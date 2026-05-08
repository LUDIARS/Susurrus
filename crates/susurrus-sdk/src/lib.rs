//! Susurrus overlay SDK。
//!
//! ホストアプリ (Pictor / Ergo / Unity 等) が組み込んで、
//! ローカル daemon (susurrus-core) に loopback HTTP/WS で接続する薄い client。
//!
//! v0.5 で Rust crate、 v0.6 で C ABI を切る。

pub struct Susurrus {
    pub endpoint: String,
}

impl Susurrus {
    pub fn local_default() -> Self {
        Self { endpoint: "http://127.0.0.1:17370".into() }
    }
}
