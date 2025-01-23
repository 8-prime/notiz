use tauri::{window, Manager};

use crate::database::DatabaseUuid;

pub fn close_search_window(handle: &tauri::AppHandle) -> Result<(), String> {
    if let Some(search_window) = handle.get_webview_window("search") {
        search_window.close().map_err(|err| err.to_string())?;
    }
    Ok(())
}
pub fn close_main_window(handle: &tauri::AppHandle) -> Result<(), String> {
    if let Some(search_window) = handle.get_webview_window("main") {
        search_window.close().map_err(|err| err.to_string())?;
    }
    Ok(())
}

pub fn open_main_window(handle: &tauri::AppHandle) -> Result<(), String> {
    let window =
        tauri::WebviewWindowBuilder::new(handle, "main", tauri::WebviewUrl::App("/".into()))
            .decorations(false)
            .title("main")
            .inner_size(600.0, 400.0)
            .focused(true)
            .center()
            .visible(false)
            .always_on_top(true)
            .build()
            .map_err(|err| err.to_string())?;
    // window.show().map_err(|err| err.to_string())?;
    Ok(())
}

pub fn open_main_window_with_id(handle: &tauri::AppHandle, id: DatabaseUuid) -> Result<(), String> {
    let window = tauri::WebviewWindowBuilder::new(
        handle,
        "main",
        tauri::WebviewUrl::App(format!("/{}", id).into()),
    )
    .decorations(false)
    .title("main")
    .inner_size(600.0, 400.0)
    .focused(true)
    .center()
    .visible(false)
    .always_on_top(true)
    .build()
    .map_err(|err| err.to_string())?;
    // window.show().map_err(|err| err.to_string())?;
    Ok(())
}

pub fn open_search_window(handle: &tauri::AppHandle) -> Result<(), String> {
    let window = tauri::WebviewWindowBuilder::new(
        handle,
        "search",
        tauri::WebviewUrl::App("/search".into()),
    )
    .decorations(false)
    .title("search")
    .inner_size(360.0, 600.0)
    .focused(true)
    .center()
    .visible(false)
    .always_on_top(true)
    .build()
    .map_err(|err| err.to_string())?;
    // window.show().map_err(|err| err.to_string())?;
    Ok(())
}
