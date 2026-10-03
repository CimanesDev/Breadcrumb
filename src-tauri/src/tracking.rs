use notify::{event::{ModifyKind, RenameMode}, EventKind, RecursiveMode, Watcher};
use std::{path::Path, sync::{mpsc, Arc, Mutex}, thread, time::Duration};
use tauri::{AppHandle, Emitter};

fn ignored(path: &Path) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    name.starts_with("~$") || [".tmp", ".crdownload", ".part", ".lock"].iter().any(|x| name.ends_with(x))
}

pub fn start(db: Arc<Mutex<rusqlite::Connection>>, app: AppHandle) -> Result<(), String> {
    let downloads = dirs::download_dir().ok_or("Downloads folder not found")?;
    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx).map_err(|e| e.to_string())?;
    watcher.watch(&downloads, RecursiveMode::Recursive).map_err(|e| e.to_string())?;
    thread::spawn(move || {
        let _watcher = watcher;
        while let Ok(result) = rx.recv() {
            let Ok(event) = result else { continue };
            if event.paths.iter().any(|p| ignored(p)) { continue }
            // A short delay lets browsers finish writing Zone.Identifier and the final file.
            thread::sleep(Duration::from_millis(400));
            let Ok(mut conn) = db.lock() else { continue };
            let outcome = match event.kind {
                EventKind::Modify(ModifyKind::Name(RenameMode::Both)) if event.paths.len() == 2 =>
                    crate::database::rename(&mut conn, &event.paths[0], &event.paths[1]),
                EventKind::Create(_) | EventKind::Modify(_) => {
                    for path in &event.paths { if path.exists() { let _ = crate::database::observe(&mut conn, path); } }
                    Ok(())
                }
                EventKind::Remove(_) => {
                    for path in &event.paths { let _ = crate::database::remove(&mut conn, path); }
                    Ok(())
                }
                _ => Ok(())
            };
            if outcome.is_ok() { let _ = app.emit("history-changed", ()); }
        }
    });
    Ok(())
}
