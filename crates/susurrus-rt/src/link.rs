//! peer の接続状態 (ACTIVE / SLEEP) を保持する小さな state machine。
//!
//! 単純な「最終 seen 時刻 + idle 閾値」 で判定する。
//! 実トランスポートに依存しないため pure module。

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    /// QUIC stream で realtime に喋れる
    Active,
    /// オフライン or NAT 切断。 Synergos chain での async に fall back
    Sleep,
}

#[derive(Debug, Clone)]
pub struct PeerLink {
    pub peer_id: String,
    pub state: LinkState,
    pub last_seen: DateTime<Utc>,
    pub last_rtt_ms: Option<u32>,
}

impl PeerLink {
    pub fn new(peer_id: impl Into<String>) -> Self {
        Self {
            peer_id: peer_id.into(),
            state: LinkState::Sleep,
            last_seen: Utc::now(),
            last_rtt_ms: None,
        }
    }

    /// 受信イベントで last_seen を更新し、 ACTIVE に格上げ。
    pub fn touch(&mut self, rtt_ms: Option<u32>) {
        self.last_seen = Utc::now();
        self.last_rtt_ms = rtt_ms.or(self.last_rtt_ms);
        self.state = LinkState::Active;
    }

    /// idle 判定 + 必要なら SLEEP に降格。
    pub fn sweep(&mut self, idle_threshold: Duration) {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(self.last_seen);
        if elapsed.num_milliseconds() > idle_threshold.as_millis() as i64 {
            self.state = LinkState::Sleep;
        }
    }
}

/// per-peer 状態の単純レジストリ。 内部は HashMap、 Tokio から外で wrap する。
#[derive(Debug, Default)]
pub struct LinkRegistry {
    peers: HashMap<String, PeerLink>,
}

impl LinkRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn touch(&mut self, peer_id: &str, rtt_ms: Option<u32>) {
        let entry = self
            .peers
            .entry(peer_id.to_string())
            .or_insert_with(|| PeerLink::new(peer_id));
        entry.touch(rtt_ms);
    }

    pub fn get(&self, peer_id: &str) -> Option<&PeerLink> {
        self.peers.get(peer_id)
    }

    pub fn sweep(&mut self, idle_threshold: Duration) -> Vec<String> {
        let mut downgraded = Vec::new();
        for (id, p) in self.peers.iter_mut() {
            let prev = p.state;
            p.sweep(idle_threshold);
            if prev == LinkState::Active && p.state == LinkState::Sleep {
                downgraded.push(id.clone());
            }
        }
        downgraded
    }

    pub fn active_peers(&self) -> Vec<&PeerLink> {
        self.peers
            .values()
            .filter(|p| p.state == LinkState::Active)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_promotes_to_active() {
        let mut p = PeerLink::new("peer-a");
        assert_eq!(p.state, LinkState::Sleep);
        p.touch(Some(42));
        assert_eq!(p.state, LinkState::Active);
        assert_eq!(p.last_rtt_ms, Some(42));
    }

    #[test]
    fn sweep_demotes_when_idle() {
        let mut p = PeerLink::new("peer-a");
        p.touch(None);
        // 過去にずらす
        p.last_seen = Utc::now() - chrono::Duration::seconds(60);
        p.sweep(Duration::from_secs(30));
        assert_eq!(p.state, LinkState::Sleep);
    }

    #[test]
    fn registry_active_list() {
        let mut r = LinkRegistry::new();
        r.touch("a", Some(10));
        r.touch("b", None);
        let actives = r.active_peers();
        assert_eq!(actives.len(), 2);
    }

    #[test]
    fn registry_sweep_returns_downgraded() {
        let mut r = LinkRegistry::new();
        r.touch("a", None);
        r.peers.get_mut("a").unwrap().last_seen = Utc::now() - chrono::Duration::seconds(60);
        let down = r.sweep(Duration::from_secs(30));
        assert_eq!(down, vec!["a".to_string()]);
    }
}
