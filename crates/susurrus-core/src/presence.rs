//! presence + typing の小さな state。 SQLite の presence/typing 表に書く。

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceState {
    Active,
    Idle,
    Offline,
}

impl PresenceState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Idle => "idle",
            Self::Offline => "offline",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Typing {
    pub thread_id: String,
    pub user_uri: String,
    pub until: DateTime<Utc>,
}
