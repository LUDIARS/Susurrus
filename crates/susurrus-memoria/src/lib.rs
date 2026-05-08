//! Memoria 連携 (opt-out 可、 既定 on)。
//!
//! - /save-to-memoria — chat msg を Memoria ブクマに送る
//! - /dig <url>       — Memoria Dig 結果を thread に返す
//! - メンション → Memoria task delegation (将来)

pub struct MemoriaClient {
    pub endpoint: String,
    pub token: String,
}

impl MemoriaClient {
    pub fn new(endpoint: impl Into<String>, token: impl Into<String>) -> Self {
        Self { endpoint: endpoint.into(), token: token.into() }
    }
}
