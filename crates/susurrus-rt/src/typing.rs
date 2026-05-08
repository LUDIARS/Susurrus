//! typing indicator の TTL 管理。
//!
//! 送信側: ある thread に対して 2 秒間隔で `until_ms = now + 3000` を送り続ける。
//! 受信側: `(thread, user_uri)` で記録し、 `until_ms <= now` で表示を消す。
//!
//! 本モジュールは pure (Tokio time のみ依存)。 SQLite への永続化は susurrus-core 側。

use crate::payload::SusTyping;
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct TypingTracker {
    /// (thread_id, user_uri) → until_ms
    inner: HashMap<(Uuid, String), i64>,
}

impl TypingTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// 受信した typing event を記録。
    pub fn observe(&mut self, t: &SusTyping) {
        self.inner
            .insert((t.thread_id, t.user_uri.clone()), t.until_ms);
    }

    /// 現在 typing 中の (thread_id, user_uri) リスト。 期限切れは含まない。
    pub fn current(&self) -> Vec<(Uuid, String)> {
        let now = Utc::now().timestamp_millis();
        self.inner
            .iter()
            .filter(|(_, &until)| until > now)
            .map(|((t, u), _)| (*t, u.clone()))
            .collect()
    }

    /// 特定 thread で typing 中の user 一覧。
    pub fn typing_in(&self, thread_id: Uuid) -> Vec<String> {
        let now = Utc::now().timestamp_millis();
        self.inner
            .iter()
            .filter(|((t, _), &until)| *t == thread_id && until > now)
            .map(|((_, u), _)| u.clone())
            .collect()
    }

    /// 期限切れのエントリを掃除。 戻り値は削除件数。
    pub fn sweep(&mut self) -> usize {
        let now = Utc::now().timestamp_millis();
        let before = self.inner.len();
        self.inner.retain(|_, until| *until > now);
        before - self.inner.len()
    }
}

/// 送信側ヘルパ: 「今 typing 中」 として送るペイロードを組み立てる。
/// `extend_ms` は until までの猶予 (typical 3000)。
pub fn build_typing(thread_id: Uuid, user_uri: &str, extend_ms: i64) -> SusTyping {
    SusTyping {
        thread_id,
        user_uri: user_uri.to_string(),
        until_ms: Utc::now().timestamp_millis() + extend_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_then_current() {
        let mut t = TypingTracker::new();
        let thr = Uuid::now_v7();
        t.observe(&build_typing(thr, "cr:a", 3000));
        let cur = t.current();
        assert_eq!(cur.len(), 1);
        assert_eq!(cur[0].1, "cr:a");
    }

    #[test]
    fn expired_dropped_by_sweep() {
        let mut t = TypingTracker::new();
        let thr = Uuid::now_v7();
        t.observe(&SusTyping {
            thread_id: thr,
            user_uri: "cr:past".into(),
            until_ms: Utc::now().timestamp_millis() - 1000, // 過去
        });
        assert_eq!(t.current().len(), 0); // 既に表示対象外
        let removed = t.sweep();
        assert_eq!(removed, 1);
    }

    #[test]
    fn typing_in_filters_by_thread() {
        let mut t = TypingTracker::new();
        let a = Uuid::now_v7();
        let b = Uuid::now_v7();
        t.observe(&build_typing(a, "cr:user1", 3000));
        t.observe(&build_typing(b, "cr:user2", 3000));
        assert_eq!(t.typing_in(a), vec!["cr:user1".to_string()]);
        assert_eq!(t.typing_in(b), vec!["cr:user2".to_string()]);
    }
}
