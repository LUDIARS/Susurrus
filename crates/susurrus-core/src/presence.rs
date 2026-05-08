//! presence + typing。 SQLite の presence/typing 表に書く + susurrus-rt の TypingTracker と同期。

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::Serialize;
use susurrus_rt::payload::SusTyping;
use susurrus_rt::typing::{build_typing, TypingTracker};
use uuid::Uuid;

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

#[derive(Debug, Clone, Serialize)]
pub struct TypingRow {
    pub thread_id: String,
    pub user_uri: String,
    pub until_ms: i64,
}

/// SQLite typing 表に upsert + tracker にも記録 (両方の真実に同期)。
pub fn record_typing(
    conn: &Connection,
    tracker: &mut TypingTracker,
    t: &SusTyping,
) -> rusqlite::Result<()> {
    tracker.observe(t);
    let until_iso = chrono::DateTime::<Utc>::from_timestamp_millis(t.until_ms)
        .unwrap_or_else(Utc::now)
        .to_rfc3339();
    conn.execute(
        "INSERT INTO typing(thread_id, user_uri, until) VALUES (?1, ?2, ?3)
         ON CONFLICT(thread_id, user_uri) DO UPDATE SET until = excluded.until",
        params![t.thread_id.to_string(), t.user_uri, until_iso],
    )?;
    Ok(())
}

/// 「今 typing 中」の一覧。 期限切れは含めない。
pub fn list_typing(conn: &Connection, thread_id: &str) -> rusqlite::Result<Vec<TypingRow>> {
    let now_iso = Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "SELECT thread_id, user_uri, until FROM typing
         WHERE thread_id = ?1 AND until > ?2",
    )?;
    let rows = stmt
        .query_map(params![thread_id, now_iso], |row| {
            let thread_id: String = row.get(0)?;
            let user_uri: String = row.get(1)?;
            let until_iso: String = row.get(2)?;
            let until_ms = DateTime::parse_from_rfc3339(&until_iso)
                .map(|d| d.timestamp_millis())
                .unwrap_or(0);
            Ok(TypingRow { thread_id, user_uri, until_ms })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// 期限切れ行を削除 (定期 sweep)。
pub fn sweep_typing(conn: &Connection, tracker: &mut TypingTracker) -> rusqlite::Result<usize> {
    let n = tracker.sweep();
    let now_iso = Utc::now().to_rfc3339();
    conn.execute("DELETE FROM typing WHERE until <= ?1", params![now_iso])?;
    Ok(n)
}

/// 自分が typing 中であることを記録 + Synergos backend へ broadcast すべきペイロードを返す。
pub fn local_start_typing(
    conn: &Connection,
    tracker: &mut TypingTracker,
    thread_id: Uuid,
    user_uri: &str,
    extend_ms: i64,
) -> rusqlite::Result<SusTyping> {
    let p = build_typing(thread_id, user_uri, extend_ms);
    record_typing(conn, tracker, &p)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use rusqlite::Connection;
    use susurrus_rt::typing::TypingTracker;

    fn fresh_db() -> Db {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        drop(tmp); // release lock
        Db::open(&path).unwrap()
    }

    #[test]
    fn record_and_list() {
        let db = fresh_db();
        let mut tracker = TypingTracker::new();
        let thr = Uuid::now_v7();
        let _ = local_start_typing(&db.conn, &mut tracker, thr, "cr:a", 3000).unwrap();
        let rows = list_typing(&db.conn, &thr.to_string()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].user_uri, "cr:a");
    }

    #[test]
    fn sweep_drops_expired() {
        let db = fresh_db();
        let mut tracker = TypingTracker::new();
        let thr = Uuid::now_v7();
        // 過去の until を直接挿入
        let past = SusTyping {
            thread_id: thr,
            user_uri: "cr:past".into(),
            until_ms: Utc::now().timestamp_millis() - 5_000,
        };
        record_typing(&db.conn, &mut tracker, &past).unwrap();
        // list_typing は既に空 (until > now で filter)
        assert_eq!(list_typing(&db.conn, &thr.to_string()).unwrap().len(), 0);
        let n = sweep_typing(&db.conn, &mut tracker).unwrap();
        assert_eq!(n, 1);
    }
}
