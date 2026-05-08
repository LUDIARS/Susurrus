//! Susurrus Tauri shell。 IPC command を susurrus-core に橋渡しする。

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping])
        .run(tauri::generate_context!())
        .expect("error while running susurrus-tauri");
}

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}
