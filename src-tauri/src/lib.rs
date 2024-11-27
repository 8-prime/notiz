use database::{DatabaseUuid, Note};
use tauri::{tray::TrayIconBuilder, Manager, State, Window};
use tokio::sync::Mutex;

mod database;

#[tauri::command]
fn minimize_window(window: Window) {
    window.minimize().unwrap();
}

#[tauri::command]
fn maximize_window(window: Window) {
    window.maximize().unwrap();
}

#[tauri::command]
fn close_window(window: Window) {
    window.close().unwrap();
}

#[tauri::command]
async fn changes(data: Note, state: State<'_, Mutex<AppState>>) -> Result<Note, String> {
    let state = state.lock().await;
    let rw = state.db.rw_transaction().map_err(|err| err.to_string())?;

    if data.id.is_none() {
        println!("Changes called with new note");
        let note = Note {
            id: Some(DatabaseUuid::new()),
            ..data
        };
        rw.insert(note.clone()).map_err(|err| err.to_string())?;
        rw.commit().map_err(|err| {
            "Failed to update article: Could not commit transaction ({err})".to_string()
        })?;
        return Ok(note);
    }
    println!("Changes called with existing note");

    let old_note = rw
        .get()
        .primary::<Note>(data.id)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "Note not found".to_string())?;
    rw.update(old_note, data.clone())
        .map_err(|err| err.to_string())?;
    rw.commit().map_err(|err| {
        "Failed to update article: Could not commit transaction ({err})".to_string()
    })?;

    Ok(data)
}

struct AppState {
    db: native_db::Database<'static>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            minimize_window,
            maximize_window,
            close_window,
            changes
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                let binding = handle.path().app_data_dir().unwrap();
                let app_data_dir = binding.to_str().unwrap();
                let db = database::init(app_data_dir).await.unwrap();
                handle.manage(Mutex::new(AppState { db }));
            });

            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::{
                    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
                };
                let alt_n_shortcut = Shortcut::new(Some(Modifiers::ALT), Code::KeyN);
                let alt_m_shortcut = Shortcut::new(Some(Modifiers::ALT), Code::KeyM);
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |_app, shortcut, event| {
                            println!("{:?}", shortcut);
                            if shortcut == &alt_n_shortcut {
                                match event.state() {
                                    ShortcutState::Released => {
                                        let webview_window = tauri::WebviewWindowBuilder::new(
                                            _app,
                                            "main",
                                            tauri::WebviewUrl::App("/".into()),
                                        )
                                        .decorations(false)
                                        .build();
                                    }
                                    _ => {}
                                }
                            }

                            if shortcut == &alt_m_shortcut {
                                match event.state() {
                                    ShortcutState::Released => {
                                        let webview_window = tauri::WebviewWindowBuilder::new(
                                            _app,
                                            "search",
                                            tauri::WebviewUrl::App("/search".into()),
                                        )
                                        .decorations(false)
                                        .build();
                                    }
                                    _ => {}
                                }
                            }
                        })
                        .build(),
                )?;

                app.global_shortcut().register(alt_n_shortcut)?;
                app.global_shortcut().register(alt_m_shortcut)?;
            }
            let tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .on_tray_icon_event(|tray, event| match event {
                    tauri::tray::TrayIconEvent::Click { .. } => {
                        println!("left click pressed and released");
                        let app = tray.app_handle();
                        let webview_window = tauri::WebviewWindowBuilder::new(
                            app,
                            "main",
                            tauri::WebviewUrl::App("index.html".into()),
                        )
                        .decorations(false)
                        .build();
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app_handle, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                api.prevent_exit();
            }
            _ => {}
        });
}
