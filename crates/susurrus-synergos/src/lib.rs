//! Susurrus ↔ Synergos IPC bridge。
//!
//! v0.3 のスコープ = **SLEEP 経路 (chain commit + auto-pull)** のみ。
//! ACTIVE 時の SUM1/SUT1/SUR1/SUX1/SUP1 stream を Synergos QUIC 上で流すには
//! Synergos の `dispatch_peer_streams` (synergos-core/src/daemon.rs) に拡張点を
//! 追加する必要がある (今 Synergos に PR を提出するスコープ外、 README に追記済)。
//!
//! 役割:
//! - synergos-core daemon との接続 (named pipe / UDS)
//! - Susurrus 用 project の Open / 維持
//! - md ファイル変更 → `IpcCommand::PublishUpdate`
//! - ファイル受信イベント (`TransferCompleted`) → 上位層へ通知
//!
//! 上位 (susurrus-core) は本 crate の `SynergosBridge` を Box<dyn> で受け取る想定。

pub mod backend;
pub mod bridge;

pub use backend::{NoopBackend, SynergosBackend};
pub use bridge::{BridgeError, IncomingFile, SynergosBridge, SynergosConfig};
