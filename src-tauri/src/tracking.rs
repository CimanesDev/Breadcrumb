use notify::{
    event::{ModifyKind, RenameMode},
    EventKind, RecursiveMode, Watcher,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};

fn ignored(path: &Path) -> bool {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    name.starts_with("~$")
        || [".tmp", ".crdownload", ".part", ".lock"]
            .iter()
            .any(|x| name.ends_with(x))
        || path.components().any(|c| {
            ["node_modules", ".git", "__pycache__", ".cache"]
                .contains(&c.as_os_str().to_string_lossy().to_lowercase().as_str())
        })
}

pub fn default_folders() -> Vec<std::path::PathBuf> {
    [
        dirs::download_dir(),
        dirs::desktop_dir(),
        dirs::document_dir(),
        dirs::picture_dir(),
        dirs::video_dir(),
    ]
    .into_iter()
    .flatten()
    .filter(|p| p.is_dir())
    .collect()
}

pub fn start(db: Arc<Mutex<rusqlite::Connection>>, app: AppHandle) -> Result<(), String> {
    let folders = default_folders();
    if folders.is_empty() {
        return Err("No default watched folders found".to_string());
    }
    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx).map_err(|e| e.to_string())?;
    for folder in folders {
        watcher
            .watch(&folder, RecursiveMode::Recursive)
            .map_err(|e| format!("{}: {e}", folder.display()))?;
    }
    thread::spawn(move || {
        let _watcher = watcher;
        let mut last_observed: HashMap<PathBuf, Instant> = HashMap::new();
        let mut rename_from: Option<(PathBuf, Instant)> = None;
        while let Ok(result) = rx.recv() {
            if last_observed.len() > 4096 {
                last_observed.retain(|_, at| at.elapsed() < Duration::from_secs(60));
            }
            let Ok(event) = result else { continue };
            #[cfg(debug_assertions)]
            eprintln!("watch event: {:?} {:?}", event.kind, event.paths);
            if event.paths.last().is_some_and(|p| ignored(p)) {
                if matches!(
                    event.kind,
                    EventKind::Modify(ModifyKind::Name(RenameMode::From))
                ) {
                    rename_from = None;
                }
                continue;
            }
            // A short delay lets browsers finish writing Zone.Identifier and the final file.
            thread::sleep(Duration::from_millis(400));
            let Ok(mut conn) = db.lock() else { continue };
            let outcome = match event.kind {
                EventKind::Modify(ModifyKind::Name(RenameMode::Both)) if event.paths.len() == 2 => {
                    if ignored(&event.paths[0]) {
                        crate::database::observe(&mut conn, &event.paths[1])
                    } else {
                        crate::database::rename(&mut conn, &event.paths[0], &event.paths[1])
                    }
                }
                EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                    rename_from = event.paths.first().map(|p| (p.clone(), Instant::now()));
                    Ok(())
                }
                EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                    if let Some(new) = event.paths.first() {
                        if let Some((old, _at)) = rename_from
                            .take()
                            .filter(|(_, at)| at.elapsed() < Duration::from_secs(5))
                        {
                            if ignored(&old) {
                                crate::database::observe(&mut conn, new)
                            } else {
                                crate::database::rename(&mut conn, &old, new)
                            }
                        } else {
                            crate::database::observe(&mut conn, new)
                        }
                    } else {
                        Ok(())
                    }
                }
                EventKind::Create(_) | EventKind::Modify(_) => {
                    for path in &event.paths {
                        if !path.exists() || ignored(path) {
                            continue;
                        }
                        if last_observed
                            .get(path)
                            .is_some_and(|at| at.elapsed() < Duration::from_secs(2))
                        {
                            continue;
                        }
                        last_observed.insert(path.clone(), Instant::now());
                        let _ = crate::database::observe(&mut conn, path);
                    }
                    Ok(())
                }
                EventKind::Remove(_) => {
                    for path in &event.paths {
                        let _ = crate::database::remove(&mut conn, path);
                    }
                    Ok(())
                }
                _ => Ok(()),
            };
            if outcome.is_ok() {
                let _ = app.emit("history-changed", ());
            }
        }
    });
    Ok(())
}
