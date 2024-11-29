// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![feature(iterator_try_collect)]

mod commands;
mod database;
mod models;
mod windows;

#[tokio::main]
async fn main() {
    notiz_lib::run().await;
}
