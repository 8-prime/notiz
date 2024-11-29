use tauri::{State, Window};
use tokio::sync::Mutex;

use crate::{
    database::{DatabaseUuid, Note},
    models::AppState,
    windows::{close_search_window, open_main_window_with_id},
};

#[tauri::command]
pub fn minimize_window(window: Window) {
    window.minimize().unwrap();
}

#[tauri::command]
pub fn maximize_window(window: Window) {
    window.maximize().unwrap();
}

#[tauri::command]
pub fn close_window(window: Window) {
    window.close().unwrap();
}

#[tauri::command]
pub async fn open_article(id: DatabaseUuid, handle: tauri::AppHandle) -> Result<(), String> {
    open_main_window_with_id(&handle, id)?;
    close_search_window(&handle)?;
    Ok(())
}

#[tauri::command]
pub async fn get_note(
    id: Option<DatabaseUuid>,
    state: State<'_, Mutex<AppState>>,
) -> Result<Note, String> {
    let state = state.lock().await;
    let r = state.db.r_transaction().map_err(|err| err.to_string())?;
    println!("get_note called with id: {:?}", id);
    let note = r
        .get()
        .primary::<Note>(id)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "Note not found".to_string())?;
    Ok(note)
}

#[tauri::command]
pub async fn get_notes(state: State<'_, Mutex<AppState>>) -> Result<Vec<Note>, String> {
    let state = state.lock().await;
    let r = state.db.r_transaction().map_err(|err| err.to_string())?;

    let notes = r
        .scan()
        .primary::<Note>()
        .map_err(|err| err.to_string())?
        .all()
        .map_err(|err| err.to_string())?
        .try_collect::<Vec<Note>>()
        .map_err(|err| err.to_string())?;
    Ok(notes)
}

#[tauri::command]
pub async fn changes(data: Note, state: State<'_, Mutex<AppState>>) -> Result<Note, String> {
    let state = state.lock().await;
    let rw = state.db.rw_transaction().map_err(|err| err.to_string())?;

    if data.id.is_none() {
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
    rw.commit().map_err(|_| {
        "Failed to update article: Could not commit transaction ({err})".to_string()
    })?;

    Ok(data)
}
