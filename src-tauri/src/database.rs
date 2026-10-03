use rusqlite::{params, Connection, Result};
use serde::Serialize;
use std::{fs, path::Path};

#[derive(Serialize)]
pub struct FileRecord {
    pub id: i64, pub name: String, pub original_name: String, pub path: String,
    pub original_path: String, pub size_bytes: i64, pub first_seen_at: String,
    pub last_seen_at: String, pub is_present: bool, pub source_url: Option<String>,
    pub referrer_url: Option<String>, pub source_domain: Option<String>,
}

#[derive(Serialize)]
pub struct FileEvent {
    pub event_type: String, pub at: String, pub old_path: Option<String>, pub new_path: Option<String>,
}

pub fn open() -> Result<Connection> {
    let base = dirs::data_local_dir().unwrap_or_else(std::env::temp_dir).join("Breadcrumb");
    fs::create_dir_all(&base).map_err(|_| rusqlite::Error::InvalidPath(base.clone()))?;
    let db = Connection::open(base.join("history.db"))?;
    db.pragma_update(None, "journal_mode", "WAL")?;
    db.execute_batch(include_str!("../migrations/001_initial.sql"))?;
    Ok(db)
}

pub fn observe(db: &mut Connection, path: &Path) -> Result<()> {
    let Ok(meta) = fs::metadata(path) else { return Ok(()) };
    if !meta.is_file() { return Ok(()) }
    let path_text = path.to_string_lossy().to_string();
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let (url, referrer, domain) = crate::provenance::zone_identifier(path);
    let tx = db.transaction()?;
    let existing: Option<(i64, bool, bool)> = tx.query_row("SELECT id,is_present,source_url IS NOT NULL FROM files WHERE current_path=?1", [&path_text], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).ok();
    if let Some((id, was_present, had_source)) = existing {
        tx.execute("UPDATE files SET size_bytes=?1,last_seen_at=datetime('now'),is_present=1,deleted_at=NULL,source_url=COALESCE(source_url,?2),referrer_url=COALESCE(referrer_url,?3),source_domain=COALESCE(source_domain,?4) WHERE id=?5", params![meta.len() as i64,url,referrer,domain,id])?;
        if !was_present {
            tx.execute("INSERT INTO file_events(file_id,event_type,new_path) VALUES (?1,'RESTORED',?2)",params![id,path_text])?;
        }
        if !had_source && url.is_some() {
            tx.execute("INSERT INTO file_events(file_id,event_type,new_path) VALUES (?1,'SOURCE_IDENTIFIED',?2)",params![id,path_text])?;
        }
    } else {
        tx.execute("INSERT INTO files (current_name,original_name,current_path,original_path,size_bytes,source_url,referrer_url,source_domain) VALUES (?1,?1,?2,?2,?3,?4,?5,?6)",params![name,path_text,meta.len() as i64,url,referrer,domain])?;
        let id = tx.last_insert_rowid();
        tx.execute("INSERT INTO file_events(file_id,event_type,new_path) VALUES (?1,?2,?3)", params![id, if url.is_some() {"DOWNLOADED"} else {"FIRST_SEEN"}, path_text])?;
    }
    tx.commit()
}

pub fn remove(db: &mut Connection, path: &Path) -> Result<()> {
    let path_text = path.to_string_lossy().to_string();
    let tx = db.transaction()?;
    let id: Option<i64> = tx.query_row("SELECT id FROM files WHERE current_path=?1 AND is_present=1", [&path_text], |r| r.get(0)).ok();
    if let Some(id) = id {
        tx.execute("UPDATE files SET is_present=0,deleted_at=datetime('now') WHERE id=?1", [id])?;
        tx.execute("INSERT INTO file_events(file_id,event_type,old_path) VALUES (?1,'DELETED',?2)",params![id,path_text])?;
    }
    tx.commit()
}

pub fn rename(db: &mut Connection, old: &Path, new: &Path) -> Result<()> {
    let old_text = old.to_string_lossy().to_string();
    let new_text = new.to_string_lossy().to_string();
    let new_name = new.file_name().unwrap_or_default().to_string_lossy().to_string();
    let tx = db.transaction()?;
    let id: Option<i64> = tx.query_row("SELECT id FROM files WHERE current_path=?1", [&old_text], |r| r.get(0)).ok();
    if let Some(id) = id {
        let kind = if old.parent() == new.parent() { "RENAMED" } else { "MOVED" };
        tx.execute("UPDATE files SET current_name=?1,current_path=?2,is_present=1,deleted_at=NULL,last_seen_at=datetime('now') WHERE id=?3",params![new_name,new_text,id])?;
        tx.execute("INSERT INTO file_events(file_id,event_type,old_path,new_path) VALUES (?1,?2,?3,?4)",params![id,kind,old_text,new_text])?;
    }
    tx.commit()?;
    if id.is_none() { observe(db,new)?; }
    Ok(())
}

pub fn list_files(db: &Connection, query: &str) -> Result<Vec<FileRecord>> {
    let q = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
    let mut stmt = db.prepare("SELECT id,current_name,original_name,current_path,original_path,size_bytes,first_seen_at,last_seen_at,is_present,source_url,referrer_url,source_domain FROM files WHERE current_name LIKE ?1 ESCAPE '\\' OR original_name LIKE ?1 ESCAPE '\\' OR current_path LIKE ?1 ESCAPE '\\' OR original_path LIKE ?1 ESCAPE '\\' OR source_domain LIKE ?1 ESCAPE '\\' OR source_url LIKE ?1 ESCAPE '\\' ORDER BY last_seen_at DESC LIMIT 200")?;
    stmt.query_map([q], |r| Ok(FileRecord {id:r.get(0)?,name:r.get(1)?,original_name:r.get(2)?,path:r.get(3)?,original_path:r.get(4)?,size_bytes:r.get(5)?,first_seen_at:r.get(6)?,last_seen_at:r.get(7)?,is_present:r.get(8)?,source_url:r.get(9)?,referrer_url:r.get(10)?,source_domain:r.get(11)?}))?.collect()
}

pub fn file_events(db: &Connection, id: i64) -> Result<Vec<FileEvent>> {
    let mut stmt = db.prepare("SELECT event_type,at,old_path,new_path FROM file_events WHERE file_id=?1 ORDER BY id DESC")?;
    stmt.query_map([id], |r| Ok(FileEvent {event_type:r.get(0)?,at:r.get(1)?,old_path:r.get(2)?,new_path:r.get(3)?}))?.collect()
}
