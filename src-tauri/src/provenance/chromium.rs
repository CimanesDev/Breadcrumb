use rusqlite::{Connection, OpenFlags};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct DownloadMatch {
    pub url: String,
    pub referrer: Option<String>,
    pub domain: Option<String>,
    pub browser: String,
    pub profile: String,
    pub confidence: String,
}

pub fn chromium_time(microseconds: i64) -> i64 {
    microseconds / 1_000_000 - 11_644_473_600
}

pub fn match_download(path: &Path, size: u64) -> Option<DownloadMatch> {
    let local = dirs::data_local_dir()?;
    let roots = [
        ("Google Chrome", local.join("Google/Chrome/User Data")),
        ("Microsoft Edge", local.join("Microsoft/Edge/User Data")),
        ("Brave", local.join("BraveSoftware/Brave-Browser/User Data")),
    ];
    for (browser, root) in roots {
        let Ok(profiles) = fs::read_dir(root) else {
            continue;
        };
        for entry in profiles.flatten() {
            let profile = entry.file_name().to_string_lossy().to_string();
            if profile != "Default" && !profile.starts_with("Profile ") {
                continue;
            }
            if let Some(mut found) = read_profile(&entry.path().join("History"), path, size) {
                found.browser = browser.to_string();
                found.profile = profile;
                return Some(found);
            }
        }
    }
    None
}

fn read_profile(history: &Path, path: &Path, size: u64) -> Option<DownloadMatch> {
    if !history.exists() {
        return None;
    }
    let temp = tempfile::tempdir().ok()?;
    let copy = temp.path().join("History");
    fs::copy(history, &copy).ok()?;
    for ext in ["-wal", "-shm"] {
        let source = PathBuf::from(format!("{}{}", history.display(), ext));
        if source.exists() {
            let _ = fs::copy(source, temp.path().join(format!("History{ext}")));
        }
    }
    let db = Connection::open_with_flags(copy, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    let mut stmt = db.prepare("SELECT id,target_path,total_bytes,start_time,tab_referrer_url FROM downloads ORDER BY start_time DESC LIMIT 300").ok()?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })
        .ok()?;
    let mut best: Option<(i32, DownloadMatch)> = None;
    for row in rows.flatten() {
        let (id, target, bytes, started, referrer) = row;
        let exact_path = Path::new(&target)
            .to_string_lossy()
            .eq_ignore_ascii_case(&path.to_string_lossy());
        let same_name = Path::new(&target).file_name() == path.file_name();
        let same_size = bytes >= 0 && bytes as u64 == size;
        let recent = (chrono::Utc::now().timestamp() - chromium_time(started)).abs() < 86_400;
        let (score, confidence) = if exact_path && recent {
            (3, "high")
        } else if same_name && same_size && recent {
            (2, "medium")
        } else {
            continue;
        };
        let Ok(url): Result<String, _> = db.query_row(
            "SELECT url FROM downloads_url_chains WHERE id=?1 ORDER BY chain_index DESC LIMIT 1",
            [id],
            |r| r.get(0),
        ) else {
            continue;
        };
        let domain = url::Url::parse(&url)
            .ok()
            .and_then(|v| v.host_str().map(str::to_string));
        if domain.is_none() {
            continue;
        }
        let found = DownloadMatch {
            url,
            referrer: referrer.filter(|v| !v.is_empty()),
            domain,
            browser: String::new(),
            profile: String::new(),
            confidence: confidence.to_string(),
        };
        if best.as_ref().is_none_or(|(current, _)| score > *current) {
            best = Some((score, found));
        }
        if score == 3 {
            break;
        }
    }
    best.map(|(_, found)| found)
}

#[cfg(test)]
mod tests {
    use super::{chromium_time, read_profile};
    use rusqlite::{params, Connection};
    #[test]
    fn converts_chromium_epoch() {
        assert_eq!(chromium_time(11_644_473_600_000_000), 0);
    }

    #[test]
    fn exact_path_matches_final_download_url() {
        let temp = tempfile::tempdir().unwrap();
        let history = temp.path().join("History");
        let db = Connection::open(&history).unwrap();
        db.execute_batch("CREATE TABLE downloads(id INTEGER,target_path TEXT,total_bytes INTEGER,start_time INTEGER,tab_referrer_url TEXT); CREATE TABLE downloads_url_chains(id INTEGER,chain_index INTEGER,url TEXT);").unwrap();
        let target = temp.path().join("paper.pdf");
        let now = (chrono::Utc::now().timestamp() + 11_644_473_600) * 1_000_000;
        db.execute(
            "INSERT INTO downloads VALUES(1,?1,42,?2,?3)",
            params![target.to_string_lossy(), now, "https://example.com/page"],
        )
        .unwrap();
        db.execute(
            "INSERT INTO downloads_url_chains VALUES(1,0,'https://example.com/redirect')",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO downloads_url_chains VALUES(1,1,'https://cdn.example.com/paper.pdf')",
            [],
        )
        .unwrap();
        drop(db);
        let found = read_profile(&history, &target, 42).unwrap();
        assert_eq!(found.url, "https://cdn.example.com/paper.pdf");
        assert_eq!(found.domain.as_deref(), Some("cdn.example.com"));
        assert_eq!(found.confidence, "high");
    }
}
