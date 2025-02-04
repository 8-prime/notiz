#![feature(iterator_try_collect)]

use models::AppState;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tokio::sync::Mutex;
use windows::{close_main_window, open_main_window, open_search_window};

mod commands;
mod database;
mod models;
mod windows;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::minimize_window,
            commands::maximize_window,
            commands::close_window,
            commands::changes,
            commands::get_note,
            commands::get_notes,
            commands::open_article,
            commands::delete_note
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            let autostart_manager = app.autolaunch();
            if autostart_manager.is_enabled()? {
                let _ = autostart_manager.enable();
            }

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
                                        if let None = _app.get_webview_window("main") {
                                            open_main_window(_app).unwrap();
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            if shortcut == &alt_m_shortcut {
                                match event.state() {
                                    ShortcutState::Released => {
                                        close_main_window(_app);
                                        open_search_window(_app);
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
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_i])?;

            let tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        println!("quit menu item was clicked");
                        app.exit(20);
                    }
                    _ => {
                        println!("menu item {:?} not handled", event.id);
                    }
                })
                .build(app)?;

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app_handle, event| match event {
            tauri::RunEvent::ExitRequested { api, code, .. } => {
                if let Some(c) = code {
                    if c != 20 {
                        api.prevent_exit();
                    }
                } else {
                    api.prevent_exit();
                }
            }
            _ => {}
        });
}
