//! Realtime transport (active 時)。
//!
//! 仕様: ../../../spec/PROTOCOL.md
//!
//! v0.2 のスコープ:
//! - stream magic + CBOR ペイロード型 ([`magic`], [`payload`])
//! - LinkState enum + per-peer state ([`link`])
//! - typing tracker (TTL 管理) ([`typing`])
//! - [`transport::MessageBus`] trait (Synergos backend がこれを実装する)
//!
//! v1.0+ で WebRTC datachannel + audio track を載せる予定 (`spec/SPATIAL.md`)。

pub mod link;
pub mod magic;
pub mod payload;
pub mod transport;
pub mod typing;

pub use link::LinkState;
pub use magic::Magic;
pub use transport::MessageBus;
