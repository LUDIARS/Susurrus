// Susurrus Tauri shell entry point.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    susurrus_tauri_lib::run()
}
