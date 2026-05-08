//! Susurrus Tauri shell。 IPC command + axum HTTP loopback (SDK 用) を起動。

pub mod commands;
pub mod http;
pub mod state;

use state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .try_init();

    let data_dir = resolve_data_dir();
    let state = Arc::new(AppState::open(&data_dir).expect("failed to open Susurrus data dir"));

    // SDK 用 loopback HTTP server (別 thread で tokio runtime を起動)
    {
        let app = state.clone();
        let port: u16 = std::env::var("SUSURRUS_LOCAL_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(17370);
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("susurrus http: tokio runtime");
            rt.block_on(async move {
                if let Err(e) = http::serve(app, port).await {
                    tracing::error!("susurrus http server exited: {e:#}");
                }
            });
        });
    }

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
            commands::start_typing,
            commands::list_typing,
            commands::current_user,
            commands::save_to_memoria,
            commands::memoria_dig,
            commands::memoria_enabled,
            commands::open_thread_window,
            commands::close_window,
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
