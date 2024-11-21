use tauri::{tray::TrayIconBuilder, Window};

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
fn changes(value: &str) {
    println!("Changes: {}", value);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    let db = database::init().await.unwrap();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            minimize_window,
            maximize_window,
            close_window,
            changes
        ])
        .setup(|app| {
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::{
                    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
                };
                let ctrl_n_shortcut = Shortcut::new(Some(Modifiers::ALT), Code::KeyN);
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |_app, shortcut, event| {
                            println!("{:?}", shortcut);
                            if shortcut == &ctrl_n_shortcut {
                                match event.state() {
                                    ShortcutState::Released => {
                                        let webview_window = tauri::WebviewWindowBuilder::new(
                                            _app,
                                            "main",
                                            tauri::WebviewUrl::App("index.html".into()),
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

                app.global_shortcut().register(ctrl_n_shortcut)?;
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
