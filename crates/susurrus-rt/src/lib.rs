//! Realtime transport (active 時)。
//!
//! 仕様: ../../../spec/PROTOCOL.md
//!
//! 役割:
//! - Synergos relay 経由 SDP/ICE 交換 (WS signaling)
//! - WebRTC PeerConnection + datachannel 確立
//! - datachannel 上で SUMS / SUTY / SURD / SURX / SUPN を送受信
//! - state machine: ACTIVE / SLEEP

pub mod magic {
    /// message commit notification
    pub const MS: &[u8; 4] = b"SUMS";
    /// typing
    pub const TY: &[u8; 4] = b"SUTY";
    /// read cursor
    pub const RD: &[u8; 4] = b"SURD";
    /// reaction
    pub const RX: &[u8; 4] = b"SURX";
    /// presence ping
    pub const PN: &[u8; 4] = b"SUPN";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    Active,
    Sleep,
}
