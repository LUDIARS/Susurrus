//! SQLite から read する高レベル API。 Tauri layer から呼ぶ想定。

use rusqlite::{params, Connection, Row};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ForumRow {
    pub id: String,
    pub path: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub visibility: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelRow {
    pub id: String,
    pub forum_id: String,
    pub path: String,
    pub name: String,
    pub topic: String,
    pub sort: i32,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThreadRow {
    pub id: String,
    pub channel_id: String,
    pub forum_id: String,
    pub title: String,
    pub author: String,
    pub ts: String,
    pub last_reply_ts: Option<String>,
    pub reply_count: i64,
    pub pinned: bool,
    pub locked: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplyRow {
    pub id: String,
    pub thread_id: String,
    pub parent_id: String,
    pub author: String,
    pub ts: String,
    pub edited_at: Option<String>,
    pub mentions: Vec<String>,
}

pub fn list_forums(conn: &Connection) -> rusqlite::Result<Vec<ForumRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, path, name, parent_id, visibility FROM forum ORDER BY path",
    )?;
    let rows = stmt
        .query_map([], map_forum)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn list_channels(conn: &Connection, forum_id: &str) -> rusqlite::Result<Vec<ChannelRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, forum_id, path, name, topic, sort, archived
         FROM channel WHERE forum_id = ?1 ORDER BY sort, name",
    )?;
    let rows = stmt
        .query_map([forum_id], map_channel)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn list_threads(
    conn: &Connection,
    channel_id: &str,
    limit: i64,
    offset: i64,
) -> rusqlite::Result<Vec<ThreadRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, channel_id, forum_id, title, author, ts,
                last_reply_ts, reply_count, pinned, locked
         FROM thread WHERE channel_id = ?1 AND deleted = 0
         ORDER BY pinned DESC, COALESCE(last_reply_ts, ts) DESC
         LIMIT ?2 OFFSET ?3",
    )?;
    let mut rows = stmt
        .query_map(params![channel_id, limit, offset], |row| {
            Ok(ThreadRow {
                id: row.get(0)?,
                channel_id: row.get(1)?,
                forum_id: row.get(2)?,
                title: row.get(3)?,
                author: row.get(4)?,
                ts: row.get(5)?,
                last_reply_ts: row.get(6)?,
                reply_count: row.get(7)?,
                pinned: row.get::<_, i32>(8)? != 0,
                locked: row.get::<_, i32>(9)? != 0,
                tags: Vec::new(),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    // tags
    for t in rows.iter_mut() {
        let mut s = conn.prepare("SELECT tag FROM thread_tag WHERE thread_id = ?1 ORDER BY tag")?;
        let tags: Vec<String> = s
            .query_map([&t.id], |r| r.get::<_, String>(0))?
            .filter_map(|x| x.ok())
            .collect();
        t.tags = tags;
    }
    Ok(rows)
}

pub fn list_replies(conn: &Connection, thread_id: &str) -> rusqlite::Result<Vec<ReplyRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, thread_id, parent_id, author, ts, edited_at
         FROM reply WHERE thread_id = ?1 AND deleted = 0
         ORDER BY ts ASC, id ASC",
    )?;
    let mut rows: Vec<ReplyRow> = stmt
        .query_map([thread_id], |row| {
            Ok(ReplyRow {
                id: row.get(0)?,
                thread_id: row.get(1)?,
                parent_id: row.get(2)?,
                author: row.get(3)?,
                ts: row.get(4)?,
                edited_at: row.get(5)?,
                mentions: Vec::new(),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for r in rows.iter_mut() {
        let mut s =
            conn.prepare("SELECT user_uri FROM reply_mention WHERE reply_id = ?1 ORDER BY user_uri")?;
        r.mentions = s
            .query_map([&r.id], |row| row.get::<_, String>(0))?
            .filter_map(|x| x.ok())
            .collect();
    }
    Ok(rows)
}

/// 単純な FTS 検索 (reply のみ)。 trigram tokenizer なので 3 文字以上推奨。
#[derive(Debug, Clone, Serialize)]
pub struct ReplySearchHit {
    pub reply_id: String,
    pub thread_id: String,
    pub author: String,
    pub ts: String,
    pub snippet: String,
}

pub fn search_replies(
    conn: &Connection,
    query: &str,
    limit: i64,
) -> rusqlite::Result<Vec<ReplySearchHit>> {
    let mut stmt = conn.prepare(
        "SELECT reply_id, thread_id, author, ts,
                snippet(reply_fts, 0, '<mark>', '</mark>', '…', 16) AS snip
         FROM reply_fts WHERE reply_fts MATCH ?1
         ORDER BY rank LIMIT ?2",
    )?;
    let rows = stmt
        .query_map(params![query, limit], |row| {
            Ok(ReplySearchHit {
                reply_id: row.get(0)?,
                thread_id: row.get(1)?,
                author: row.get(2)?,
                ts: row.get(3)?,
                snippet: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn map_forum(row: &Row<'_>) -> rusqlite::Result<ForumRow> {
    Ok(ForumRow {
        id: row.get(0)?,
        path: row.get(1)?,
        name: row.get(2)?,
        parent_id: row.get(3)?,
        visibility: row.get(4)?,
    })
}

fn map_channel(row: &Row<'_>) -> rusqlite::Result<ChannelRow> {
    Ok(ChannelRow {
        id: row.get(0)?,
        forum_id: row.get(1)?,
        path: row.get(2)?,
        name: row.get(3)?,
        topic: row.get(4)?,
        sort: row.get(5)?,
        archived: row.get::<_, i32>(6)? != 0,
    })
}
