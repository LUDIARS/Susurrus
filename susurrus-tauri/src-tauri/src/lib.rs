//! Susurrus Tauri shell。 IPC command を susurrus-core に橋渡しする。

mod commands;
mod state;

use state::AppState;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .try_init();

    let data_dir = resolve_data_dir();
    let state = AppState::open(&data_dir).expect("failed to open Susurrus data dir");

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::list_forums,
            commands::list_channels,
            commands::list_threads,
            commands::list_replies,
            commands::search_replies,
            commands::create_forum,
            commands::create_channel,
            commands::create_thread,
            commands::create_reply,
            commands::reindex_all,
            commands::read_thread_body,
            commands::read_reply_body,
        ])
        .run(tauri::generate_context!())
        .expect("error while running susurrus-tauri");
}

fn resolve_data_dir() -> PathBuf {
    if let Ok(p) = std::env::var("SUSURRUS_DATA") {
        return PathBuf::from(p);
    }
    // Tauri の AppData 領域を使うのが本来だが、 v0.0 は OS のローカル app data 直下に置く
    let base = dirs_next::data_local_dir()
        .or_else(dirs_next::data_dir)
        .or_else(dirs_next::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("Susurrus")
}
