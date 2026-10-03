#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod database;
mod identity;
#[cfg(not(debug_assertions))]
mod integration;
mod provenance;
mod tracking;

use std::{path::Path, sync::{Arc, Mutex}};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager, WindowEvent};

pub struct AppState {
    pub db: Arc<Mutex<rusqlite::Connection>>,
    pub pending_path: Mutex<Option<String>>,
}

#[tauri::command]
fn take_pending_path(state: tauri::State<AppState>) -> Option<String> {
    state.pending_path.lock().ok()?.take()
}

#[tauri::command]
fn inspect_file(state: tauri::State<AppState>, path: String) -> Result<Option<database::FileRecord>, String> {
    let path = Path::new(&path);
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    if path.is_file() && database::file_by_path(&db, path).map_err(|e| e.to_string())?.is_none() {
        database::observe(&mut db, path).map_err(|e| e.to_string())?;
    }
    database::file_by_path(&db, path).map_err(|e| e.to_string())
}

fn inspection_arg(args: &[String]) -> Option<String> {
    args.windows(2).find(|pair| pair[0] == "--inspect").map(|pair| pair[1].clone())
}

fn show_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn list_files(
    state: tauri::State<AppState>,
    query: String,
) -> Result<Vec<database::FileRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    database::list_files(&db, &query).map_err(|e| e.to_string())
}

#[tauri::command]
fn file_events(
    state: tauri::State<AppState>,
    file_id: i64,
) -> Result<Vec<database::FileEvent>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    database::file_events(&db, file_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_sources(state: tauri::State<AppState>) -> Result<Vec<database::SourceRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    database::list_sources(&db).map_err(|e| e.to_string())
}

#[tauri::command]
fn recent_activity(state: tauri::State<AppState>) -> Result<Vec<database::ActivityRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    database::recent_activity(&db).map_err(|e| e.to_string())
}

#[tauri::command]
fn watched_folders() -> Vec<String> {
    tracking::default_folders()
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect()
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(path) = inspection_arg(&args) {
                if let Some(state) = app.try_state::<AppState>() {
                    if let Ok(mut pending) = state.pending_path.lock() {
                        *pending = Some(path.clone());
                    }
                }
                let _ = app.emit("inspect-path", path);
            }
            if !args.iter().any(|arg| arg == "--background") {
                show_window(app);
            }
        }))
        .setup(|app| {
            let db = database::open()?;
            let args: Vec<String> = std::env::args().collect();
            let pending_path = inspection_arg(&args);
            let state = AppState {
                db: Arc::new(Mutex::new(db)),
                pending_path: Mutex::new(pending_path.clone()),
            };
            tracking::start(state.db.clone(), app.handle().clone())
                .map_err(std::io::Error::other)?;
            app.manage(state);

            #[cfg(all(windows, not(debug_assertions)))]
            if let Err(error) = integration::register() {
                eprintln!("Could not register Explorer menu or startup: {error}");
            }

            if pending_path.is_some() || (!args.iter().any(|arg| arg == "--background") && cfg!(debug_assertions)) {
                show_window(app.handle());
            }

            let open = MenuItem::with_id(app, "open", "Open Breadcrumb", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            TrayIconBuilder::new()
                .icon(
                    app.default_window_icon()
                        .cloned()
                        .expect("application icon"),
                )
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        show_window(app);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_files,
            inspect_file,
            take_pending_path,
            file_events,
            list_sources,
            recent_activity,
            watched_folders
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Breadcrumb");
}
