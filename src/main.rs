mod app;
mod semantic;
mod storage;
mod tray;

use std::{error::Error, path::PathBuf};

use directories::ProjectDirs;
use eframe::egui;

fn main() -> Result<(), Box<dyn Error>> {
    let data_dir = match std::env::var_os("NOTIZ_DATA_DIR") {
        Some(path) => PathBuf::from(path),
        None => ProjectDirs::from("", "", "Notiz")
            .ok_or("Could not find a local application data directory")?
            .data_local_dir()
            .to_path_buf(),
    };
    std::fs::create_dir_all(&data_dir)?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let store = runtime.block_on(storage::Store::open(&data_dir.join("notes.sqlite3")))?;
    let notes = runtime.block_on(store.list())?;
    let settings = runtime.block_on(store.settings())?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 520.0])
            .with_min_inner_size([320.0, 320.0])
            .with_visible(false)
            .with_taskbar(false),
        ..Default::default()
    };

    eframe::run_native(
        "Notiz",
        options,
        Box::new(move |cc| {
            app::NotizApp::new(cc, runtime, store, notes, settings)
                .map(|app| Box::new(app) as Box<dyn eframe::App>)
        }),
    )?;
    Ok(())
}
