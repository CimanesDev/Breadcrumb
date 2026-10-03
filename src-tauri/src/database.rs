use rusqlite::{params, Connection, Result};
use serde::Serialize;
use std::{fs, path::Path};

#[derive(Serialize)]
pub struct FileRecord {
    pub id: i64,
    pub name: String,
    pub original_name: String,
    pub path: String,
    pub original_path: String,
    pub size_bytes: i64,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub is_present: bool,
    pub source_url: Option<String>,
    pub source_page_url: Option<String>,
    pub referrer_url: Option<String>,
    pub source_domain: Option<String>,
    pub browser_name: Option<String>,
    pub browser_profile: Option<String>,
    pub source_confidence: Option<String>,
}

#[derive(Serialize)]
pub struct FileEvent {
    pub event_type: String,
    pub at: String,
    pub old_path: Option<String>,
    pub new_path: Option<String>,
}

#[derive(Serialize)]
pub struct SourceRecord {
    pub domain: String,
    pub file_count: i64,
    pub total_bytes: i64,
    pub latest_seen_at: String,
}

#[derive(Serialize)]
pub struct ActivityRecord {
    pub file_id: i64,
    pub name: String,
    pub source_domain: Option<String>,
    pub event_type: String,
    pub at: String,
    pub old_path: Option<String>,
    pub new_path: Option<String>,
}

pub fn open() -> Result<Connection> {
    let base = dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("Breadcrumb");
    fs::create_dir_all(&base).map_err(|_| rusqlite::Error::InvalidPath(base.clone()))?;
    let db = Connection::open(base.join("history.db"))?;
    db.pragma_update(None, "journal_mode", "WAL")?;
    db.execute_batch(include_str!("../migrations/001_initial.sql"))?;
    let migrated: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('files') WHERE name='browser_name')",
        [],
        |r| r.get(0),
    )?;
    if !migrated {
        db.execute_batch(include_str!("../migrations/002_browser.sql"))?;
    }
    let has_key: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('files') WHERE name='file_key')",
        [],
        |r| r.get(0),
    )?;
    if !has_key {
        db.execute_batch(include_str!("../migrations/003_identity.sql"))?;
    }
    let has_page: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('files') WHERE name='source_page_url')",
        [],
        |r| r.get(0),
    )?;
    if !has_page {
        db.execute_batch(include_str!("../migrations/004_source_page.sql"))?;
    }
    Ok(db)
}

pub fn observe(db: &mut Connection, path: &Path) -> Result<()> {
    observe_inner(db, path, false)
}

pub fn enrich(db: &mut Connection, path: &Path) -> Result<()> {
    observe_inner(db, path, true)
}

fn observe_inner(db: &mut Connection, path: &Path, force_browser: bool) -> Result<()> {
    let Ok(meta) = fs::metadata(path) else {
        return Ok(());
    };
    if !meta.is_file() {
        return Ok(());
    }
    let path_text = path.to_string_lossy().to_string();
    let file_key = crate::identity::file_key(path);
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let (already_sourced, already_has_page): (bool, bool) = db
        .query_row(
            "SELECT source_url IS NOT NULL,source_page_url IS NOT NULL FROM files WHERE current_path=?1",
            [&path_text],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or((false, false));
    let (mut url, mut referrer, mut domain) = crate::provenance::zone_identifier(path);
    let mut page_url = None;
    let mut browser_name = None;
    let mut browser_profile = None;
    let mut confidence = if url.is_some() {
        Some("high".to_string())
    } else {
        None
    };
    if !already_has_page && (force_browser || !already_sourced) {
        if let Some(found) = crate::provenance::chromium::match_download(path, meta.len()) {
            page_url = found.page_url;
            if url.is_none() {
                url = Some(found.url);
                referrer = found.referrer;
                domain = found.domain;
                browser_name = Some(found.browser);
                browser_profile = Some(found.profile);
                confidence = Some(found.confidence);
            } else {
                browser_name = Some(found.browser);
                browser_profile = Some(found.profile);
            }
        }
    }
    let tx = db.transaction()?;
    let existing: Option<(i64, bool, bool, bool)> = tx
        .query_row(
            "SELECT id,is_present,source_url IS NOT NULL,source_page_url IS NOT NULL FROM files WHERE current_path=?1",
            [&path_text],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok();
    if let Some((id, was_present, had_source, had_page)) = existing {
        tx.execute("UPDATE files SET size_bytes=?1,last_seen_at=datetime('now'),is_present=1,deleted_at=NULL,source_url=COALESCE(source_url,?2),referrer_url=COALESCE(referrer_url,?3),source_domain=COALESCE(source_domain,?4),browser_name=COALESCE(browser_name,?5),browser_profile=COALESCE(browser_profile,?6),source_confidence=COALESCE(source_confidence,?7),file_key=COALESCE(file_key,?8),source_page_url=COALESCE(source_page_url,?9) WHERE id=?10", params![meta.len() as i64,url,referrer,domain,browser_name,browser_profile,confidence,file_key,page_url,id])?;
        if !was_present {
            tx.execute(
                "INSERT INTO file_events(file_id,event_type,new_path) VALUES (?1,'RESTORED',?2)",
                params![id, path_text],
            )?;
        }
        if !had_source && url.is_some() {
            tx.execute("INSERT INTO file_events(file_id,event_type,new_path) VALUES (?1,'SOURCE_IDENTIFIED',?2)",params![id,path_text])?;
        }
        if !had_page && page_url.is_some() {
            tx.execute("INSERT INTO file_events(file_id,event_type,new_path) VALUES (?1,'SOURCE_PAGE_IDENTIFIED',?2)",params![id,path_text])?;
        }
    } else {
        let moved = if let Some(ref key) = file_key {
            tx.query_row("SELECT id,current_path FROM files WHERE file_key=?1 AND current_path<>?2 AND last_seen_at>=datetime('now','-1 minute') ORDER BY last_seen_at DESC LIMIT 1",params![key,path_text],|r| Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?))).ok()
                .filter(|(_,old_path)| !Path::new(old_path).exists())
        } else {
            None
        };
        if let Some((id, old_path)) = moved {
            tx.execute("UPDATE files SET current_name=?1,current_path=?2,size_bytes=?3,last_seen_at=datetime('now'),is_present=1,deleted_at=NULL,source_url=COALESCE(source_url,?4),referrer_url=COALESCE(referrer_url,?5),source_domain=COALESCE(source_domain,?6),browser_name=COALESCE(browser_name,?7),browser_profile=COALESCE(browser_profile,?8),source_confidence=COALESCE(source_confidence,?9),source_page_url=COALESCE(source_page_url,?10) WHERE id=?11",params![name,path_text,meta.len() as i64,url,referrer,domain,browser_name,browser_profile,confidence,page_url,id])?;
            tx.execute("DELETE FROM file_events WHERE id=(SELECT id FROM file_events WHERE file_id=?1 AND event_type='DELETED' ORDER BY id DESC LIMIT 1)", [id])?;
            tx.execute("INSERT INTO file_events(file_id,event_type,old_path,new_path) VALUES (?1,'MOVED',?2,?3)",params![id,old_path,path_text])?;
        } else {
            tx.execute("INSERT INTO files (current_name,original_name,current_path,original_path,size_bytes,source_url,referrer_url,source_domain,browser_name,browser_profile,source_confidence,file_key,source_page_url) VALUES (?1,?1,?2,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![name,path_text,meta.len() as i64,url,referrer,domain,browser_name,browser_profile,confidence,file_key,page_url])?;
            let id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO file_events(file_id,event_type,new_path) VALUES (?1,?2,?3)",
                params![
                    id,
                    if url.is_some() {
                        "DOWNLOADED"
                    } else {
                        "FIRST_SEEN"
                    },
                    path_text
                ],
            )?;
        }
    }
    tx.commit()
}

pub fn remove(db: &mut Connection, path: &Path) -> Result<()> {
    let path_text = path.to_string_lossy().to_string();
    let tx = db.transaction()?;
    let id: Option<i64> = tx
        .query_row(
            "SELECT id FROM files WHERE current_path=?1 AND is_present=1",
            [&path_text],
            |r| r.get(0),
        )
        .ok();
    if let Some(id) = id {
        tx.execute(
            "UPDATE files SET is_present=0,deleted_at=datetime('now'),last_seen_at=datetime('now') WHERE id=?1",
            [id],
        )?;
        tx.execute(
            "INSERT INTO file_events(file_id,event_type,old_path) VALUES (?1,'DELETED',?2)",
            params![id, path_text],
        )?;
    }
    tx.commit()
}

pub fn rename(db: &mut Connection, old: &Path, new: &Path) -> Result<()> {
    let old_text = old.to_string_lossy().to_string();
    let new_text = new.to_string_lossy().to_string();
    let new_name = new
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let tx = db.transaction()?;
    let id: Option<i64> = tx
        .query_row(
            "SELECT id FROM files WHERE current_path=?1",
            [&old_text],
            |r| r.get(0),
        )
        .ok();
    if let Some(id) = id {
        let kind = if old.parent() == new.parent() {
            "RENAMED"
        } else {
            "MOVED"
        };
        tx.execute("UPDATE files SET current_name=?1,current_path=?2,is_present=1,deleted_at=NULL,last_seen_at=datetime('now'),file_key=COALESCE(file_key,?3) WHERE id=?4",params![new_name,new_text,crate::identity::file_key(new),id])?;
        tx.execute(
            "INSERT INTO file_events(file_id,event_type,old_path,new_path) VALUES (?1,?2,?3,?4)",
            params![id, kind, old_text, new_text],
        )?;
    } else if new.is_dir() {
        let old_prefix = format!("{}\\", old_text.trim_end_matches('\\'));
        let descendants: Vec<(i64, String)> = {
            let mut stmt =
                tx.prepare("SELECT id,current_path FROM files WHERE substr(current_path,1,?1)=?2")?;
            let rows = stmt.query_map(params![old_prefix.len() as i64, old_prefix], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?;
            rows.collect::<Result<_>>()?
        };
        for (child_id, child_old) in descendants {
            let Ok(suffix) = Path::new(&child_old).strip_prefix(old) else {
                continue;
            };
            let child_new = new.join(suffix).to_string_lossy().to_string();
            tx.execute(
                "UPDATE files SET current_path=?1,last_seen_at=datetime('now') WHERE id=?2",
                params![child_new, child_id],
            )?;
            tx.execute("INSERT INTO file_events(file_id,event_type,old_path,new_path) VALUES (?1,'MOVED',?2,?3)",params![child_id,child_old,child_new])?;
        }
    }
    tx.commit()?;
    if id.is_none() {
        observe(db, new)?;
    }
    Ok(())
}

pub fn list_files(db: &Connection, query: &str) -> Result<Vec<FileRecord>> {
    let q = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
    let mut stmt = db.prepare("SELECT id,current_name,original_name,current_path,original_path,size_bytes,first_seen_at,last_seen_at,is_present,source_url,referrer_url,source_domain,browser_name,browser_profile,source_confidence,source_page_url FROM files WHERE current_name LIKE ?1 ESCAPE '\\' OR original_name LIKE ?1 ESCAPE '\\' OR current_path LIKE ?1 ESCAPE '\\' OR original_path LIKE ?1 ESCAPE '\\' OR source_domain LIKE ?1 ESCAPE '\\' OR source_url LIKE ?1 ESCAPE '\\' OR source_page_url LIKE ?1 ESCAPE '\\' OR browser_name LIKE ?1 ESCAPE '\\' OR id IN (SELECT file_id FROM file_events WHERE old_path LIKE ?1 ESCAPE '\\' OR new_path LIKE ?1 ESCAPE '\\') ORDER BY last_seen_at DESC LIMIT 200")?;
    let rows = stmt.query_map([q], |r| {
        Ok(FileRecord {
            id: r.get(0)?,
            name: r.get(1)?,
            original_name: r.get(2)?,
            path: r.get(3)?,
            original_path: r.get(4)?,
            size_bytes: r.get(5)?,
            first_seen_at: r.get(6)?,
            last_seen_at: r.get(7)?,
            is_present: r.get(8)?,
            source_url: r.get(9)?,
            referrer_url: r.get(10)?,
            source_domain: r.get(11)?,
            browser_name: r.get(12)?,
            browser_profile: r.get(13)?,
            source_confidence: r.get(14)?,
            source_page_url: r.get(15)?,
        })
    })?;
    rows.collect()
}

pub fn file_by_path(db: &Connection, path: &Path) -> Result<Option<FileRecord>> {
    let path_text = path.to_string_lossy();
    let mut stmt = db.prepare("SELECT id,current_name,original_name,current_path,original_path,size_bytes,first_seen_at,last_seen_at,is_present,source_url,referrer_url,source_domain,browser_name,browser_profile,source_confidence,source_page_url FROM files WHERE current_path=?1 COLLATE NOCASE OR original_path=?1 COLLATE NOCASE ORDER BY (current_path=?1 COLLATE NOCASE) DESC,last_seen_at DESC LIMIT 1")?;
    let mut rows = stmt.query([path_text.as_ref()])?;
    if let Some(r) = rows.next()? {
        Ok(Some(FileRecord {
            id: r.get(0)?, name: r.get(1)?, original_name: r.get(2)?,
            path: r.get(3)?, original_path: r.get(4)?, size_bytes: r.get(5)?,
            first_seen_at: r.get(6)?, last_seen_at: r.get(7)?, is_present: r.get(8)?,
            source_url: r.get(9)?, referrer_url: r.get(10)?, source_domain: r.get(11)?,
            browser_name: r.get(12)?, browser_profile: r.get(13)?, source_confidence: r.get(14)?,
            source_page_url: r.get(15)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn file_events(db: &Connection, id: i64) -> Result<Vec<FileEvent>> {
    let mut stmt = db.prepare(
        "SELECT event_type,at,old_path,new_path FROM file_events WHERE file_id=?1 ORDER BY id ASC",
    )?;
    let rows = stmt.query_map([id], |r| {
        Ok(FileEvent {
            event_type: r.get(0)?,
            at: r.get(1)?,
            old_path: r.get(2)?,
            new_path: r.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn list_sources(db: &Connection) -> Result<Vec<SourceRecord>> {
    let mut stmt = db.prepare("SELECT source_domain,COUNT(*),SUM(size_bytes),MAX(first_seen_at) FROM files WHERE source_domain IS NOT NULL GROUP BY source_domain ORDER BY COUNT(*) DESC,source_domain LIMIT 200")?;
    let rows = stmt.query_map([], |r| {
        Ok(SourceRecord {
            domain: r.get(0)?,
            file_count: r.get(1)?,
            total_bytes: r.get(2)?,
            latest_seen_at: r.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn recent_activity(db: &Connection) -> Result<Vec<ActivityRecord>> {
    let mut stmt = db.prepare("SELECT e.file_id,f.current_name,f.source_domain,e.event_type,e.at,e.old_path,e.new_path FROM file_events e JOIN files f ON f.id=e.file_id ORDER BY e.id DESC LIMIT 200")?;
    let rows = stmt.query_map([], |r| {
        Ok(ActivityRecord {
            file_id: r.get(0)?,
            name: r.get(1)?,
            source_domain: r.get(2)?,
            event_type: r.get(3)?,
            at: r.get(4)?,
            old_path: r.get(5)?,
            new_path: r.get(6)?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_identity_through_rename_move_and_delete() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first.pdf");
        fs::write(&first, b"example").unwrap();
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch(include_str!("../migrations/001_initial.sql"))
            .unwrap();
        db.execute_batch(include_str!("../migrations/002_browser.sql"))
            .unwrap();
        db.execute_batch(include_str!("../migrations/003_identity.sql"))
            .unwrap();
        db.execute_batch(include_str!("../migrations/004_source_page.sql"))
            .unwrap();
        observe(&mut db, &first).unwrap();
        let renamed = temp.path().join("renamed.pdf");
        fs::rename(&first, &renamed).unwrap();
        rename(&mut db, &first, &renamed).unwrap();
        let folder = temp.path().join("other");
        fs::create_dir(&folder).unwrap();
        let moved = folder.join("renamed.pdf");
        fs::rename(&renamed, &moved).unwrap();
        rename(&mut db, &renamed, &moved).unwrap();
        fs::remove_file(&moved).unwrap();
        remove(&mut db, &moved).unwrap();
        let files = list_files(&db, "").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].original_name, "first.pdf");
        assert!(!files[0].is_present);
        let kinds: Vec<_> = file_events(&db, files[0].id)
            .unwrap()
            .into_iter()
            .map(|e| e.event_type)
            .collect();
        assert_eq!(kinds, ["FIRST_SEEN", "RENAMED", "MOVED", "DELETED"]);
    }

    #[test]
    fn folder_move_updates_child_paths() {
        let temp = tempfile::tempdir().unwrap();
        let old_dir = temp.path().join("old");
        fs::create_dir(&old_dir).unwrap();
        let child = old_dir.join("child.txt");
        fs::write(&child, b"data").unwrap();
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch(include_str!("../migrations/001_initial.sql"))
            .unwrap();
        db.execute_batch(include_str!("../migrations/002_browser.sql"))
            .unwrap();
        db.execute_batch(include_str!("../migrations/003_identity.sql"))
            .unwrap();
        db.execute_batch(include_str!("../migrations/004_source_page.sql"))
            .unwrap();
        observe(&mut db, &child).unwrap();
        let new_dir = temp.path().join("new");
        fs::rename(&old_dir, &new_dir).unwrap();
        rename(&mut db, &old_dir, &new_dir).unwrap();
        assert_eq!(
            list_files(&db, "child").unwrap()[0].path,
            new_dir.join("child.txt").to_string_lossy()
        );
    }

    #[test]
    fn separate_remove_and_create_events_preserve_file_identity() {
        let temp = tempfile::tempdir().unwrap();
        let old = temp.path().join("old.txt");
        let folder = temp.path().join("new-folder");
        fs::create_dir(&folder).unwrap();
        fs::write(&old, b"same file").unwrap();
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch(include_str!("../migrations/001_initial.sql"))
            .unwrap();
        db.execute_batch(include_str!("../migrations/002_browser.sql"))
            .unwrap();
        db.execute_batch(include_str!("../migrations/003_identity.sql"))
            .unwrap();
        db.execute_batch(include_str!("../migrations/004_source_page.sql"))
            .unwrap();
        observe(&mut db, &old).unwrap();
        let new = folder.join("old.txt");
        fs::rename(&old, &new).unwrap();
        remove(&mut db, &old).unwrap();
        observe(&mut db, &new).unwrap();
        let files = list_files(&db, "old.txt").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, new.to_string_lossy());
        let kinds: Vec<_> = file_events(&db, files[0].id)
            .unwrap()
            .into_iter()
            .map(|e| e.event_type)
            .collect();
        assert_eq!(kinds, ["FIRST_SEEN", "MOVED"]);
    }
}
