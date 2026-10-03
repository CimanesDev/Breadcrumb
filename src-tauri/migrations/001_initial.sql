CREATE TABLE IF NOT EXISTS files (
 id INTEGER PRIMARY KEY, current_name TEXT NOT NULL, original_name TEXT NOT NULL,
 current_path TEXT NOT NULL UNIQUE, original_path TEXT NOT NULL, size_bytes INTEGER NOT NULL,
 first_seen_at TEXT NOT NULL DEFAULT (datetime('now')), last_seen_at TEXT NOT NULL DEFAULT (datetime('now')),
 deleted_at TEXT, is_present INTEGER NOT NULL DEFAULT 1,
 source_url TEXT, referrer_url TEXT, source_domain TEXT
);
CREATE INDEX IF NOT EXISTS idx_files_name ON files(current_name);
CREATE INDEX IF NOT EXISTS idx_files_domain ON files(source_domain);
CREATE INDEX IF NOT EXISTS idx_files_seen ON files(last_seen_at);
CREATE TABLE IF NOT EXISTS file_events (
 id INTEGER PRIMARY KEY, file_id INTEGER NOT NULL REFERENCES files(id),
 event_type TEXT NOT NULL, at TEXT NOT NULL DEFAULT (datetime('now')),
 old_path TEXT, new_path TEXT
);
CREATE INDEX IF NOT EXISTS idx_events_file ON file_events(file_id,id);
