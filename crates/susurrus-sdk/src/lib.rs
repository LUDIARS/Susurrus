//! Susurrus overlay SDK。
//!
//! ホストアプリ (Pictor / Ergo / Unity / 任意ゲーム) から Susurrus daemon に
//! HTTP loopback で繋ぎ、 thread を購読 / 投稿する薄いクライアント。
//!
//! 構成:
//! - [`client`] = Rust ネイティブ async API
//! - [`spatial`] = Spatial Chat (v1.0+) で位置情報を送るためのヘルパ
//! - [`abi`] = C ABI ラッパ (cdylib output `susurrus_sdk.dll/.so/.dylib`)

pub mod abi;
pub mod client;
pub mod spatial;
pub mod types;

pub use client::Susurrus;
pub use types::{ReplyView, SpatialPosition};
