use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// $SUSURRUS_DATA。 forums/ と db/ がこの下に置かれる
    pub data_dir: PathBuf,
    /// Cernere endpoint
    pub cernere_endpoint: String,
    /// loopback HTTP/IPC port (Tauri から繋ぐ)
    #[serde(default = "default_loopback_port")]
    pub loopback_port: u16,
    /// Memoria 連携 opt-out
    #[serde(default)]
    pub memoria: MemoriaConfig,
    /// typing 送信 (受信は常時)
    #[serde(default = "yes")]
    pub send_typing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoriaConfig {
    /// 既定 true。 false で完全 opt-out
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "default_memoria_endpoint")]
    pub endpoint: String,
}

fn yes() -> bool { true }
fn default_loopback_port() -> u16 { 17370 } // PORT-MAP に登録予定
fn default_memoria_endpoint() -> String { "http://127.0.0.1:5180".into() }
