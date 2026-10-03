#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod database;
mod tracking;
mod provenance;

use std::sync::{Arc, Mutex};
use tauri::{Manager, WindowEvent};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;

pub struct AppState { pub db: Arc<Mutex<rusqlite::Connection>> }

#[tauri::command]
fn list_files(state: tauri::State<AppState>, query: String) -> Result<Vec<database::FileRecord>, String> {
    database::list_files(&state.db.lock().map_err(|e| e.to_string())?, &query).map_err(|e| e.to_string())
}

#[tauri::command]
fn file_events(state: tauri::State<AppState>, file_id: i64) -> Result<Vec<database::FileEvent>, String> {
    database::file_events(&state.db.lock().map_err(|e| e.to_string())?, file_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn watched_folder() -> String { dirs::download_dir().unwrap_or_default().to_string_lossy().to_string() }

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let db = database::open()?;
            let state = AppState { db: Arc::new(Mutex::new(db)) };
            tracking::start(state.db.clone(), app.handle().clone()).map_err(std::io::Error::other)?;
            app.manage(state);

            let open = MenuItem::with_id(app, "open", "Open Breadcrumb", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().expect("application icon"))
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => { if let Some(window) = app.get_webview_window("main") { let _ = window.show(); let _ = window.set_focus(); } }
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
        .invoke_handler(tauri::generate_handler![list_files, file_events, watched_folder])
        .run(tauri::generate_context!())
        .expect("failed to run Breadcrumb");
}
