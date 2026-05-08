//! md ファイル群 → SQLite キャッシュへ反映するインデクサ。
//!
//! 入口: [`reindex_all`] = 全件 walk + upsert。
//! 差分更新: [`reindex_path`] = 単一 md だけ upsert (FS watcher から呼ぶ想定)。
//!
//! 「md は正本、 DB はキャッシュ」 の原則: db に存在しなくなった行は本関数では消さない
//! (ファイル削除は別途 [`prune_missing`] で扱う)。

use crate::db::Db;
use crate::store::MdStore;
use crate::text;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use susurrus_md::{self as md, FrontMatter, Kind};
use tracing::{debug, warn};

#[derive(Debug, Default, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct IndexStats {
    pub scanned: usize,
    pub upserted: usize,
    pub unchanged: usize,
    pub failed: usize,
}

/// 全件 reindex。 FK 制約のため kind 順 (Forum→Channel→Thread→Reply) でソートして処理する。
pub fn reindex_all(db: &mut Db, store: &MdStore) -> anyhow::Result<IndexStats> {
    let mut stats = IndexStats::default();

    // pass 1: 全 md を読み込んで (path, fm, body, hash) を集める
    let mut entries: Vec<ParsedEntry> = Vec::new();
    for path in store.walk() {
        stats.scanned += 1;
        match parse_one(&path) {
            Ok(e) => entries.push(e),
            Err(e) => {
                warn!("indexer: parse failed for {:?}: {e:#}", path);
                stats.failed += 1;
            }
        }
    }

    // pass 2: kind 順で並べる
    entries.sort_by_key(|e| kind_order(e.fm.kind()));

    // pass 3: トランザクションで一括 upsert
    let tx = db.conn.transaction()?;
    for e in entries {
        match upsert_entry(&tx, store, &e) {
            Ok(true)  => stats.upserted  += 1,
            Ok(false) => stats.unchanged += 1,
            Err(err) => {
                warn!("indexer: upsert failed for {:?}: {err:#}", e.path);
                stats.failed += 1;
            }
        }
    }
    tx.commit()?;
    Ok(stats)
}

/// 単一 md の差分 reindex (FS watcher などから呼ぶ)。 依存先 (forum/channel/thread)
/// が DB に未登録だと FK で失敗するので、 watcher 経路でも上位を先に書く運用が前提。
pub fn reindex_path(db: &mut Db, store: &MdStore, path: &Path) -> anyhow::Result<bool> {
    let entry = parse_one(path)?;
    let tx = db.conn.transaction()?;
    let changed = upsert_entry(&tx, store, &entry)?;
    tx.commit()?;
    Ok(changed)
}

struct ParsedEntry {
    path: PathBuf,
    fm: FrontMatter,
    body: String,
    md_hash: String,
    mtime: i64,
}

fn parse_one(path: &Path) -> anyhow::Result<ParsedEntry> {
    let bytes = std::fs::read(path)?;
    let text_str = String::from_utf8(bytes)?;
    let normalized = if text_str.contains('\r') { text_str.replace("\r\n", "\n") } else { text_str };
    let (fm, body) = md::parse(&normalized)?;
    let md_hash = md::hash(&normalized);
    let mtime = std::fs::metadata(path)?
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    Ok(ParsedEntry { path: path.to_path_buf(), fm, body, md_hash, mtime })
}

fn kind_order(k: Kind) -> u8 {
    match k {
        Kind::Forum   => 0,
        Kind::Channel => 1,
        Kind::Thread  => 2,
        Kind::Reply   => 3,
    }
}

fn upsert_entry(tx: &Connection, store: &MdStore, e: &ParsedEntry) -> anyhow::Result<bool> {
    let md_path = store
        .rel(&e.path)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();

    let table = match e.fm.kind() {
        Kind::Forum   => "forum",
        Kind::Channel => "channel",
        Kind::Thread  => "thread",
        Kind::Reply   => "reply",
    };
    let id = e.fm.id().to_string();
    let prev_hash: Option<String> = tx
        .query_row(
            &format!("SELECT md_hash FROM {table} WHERE id = ?1"),
            params![id],
            |row| row.get(0),
        )
        .ok();
    if prev_hash.as_deref() == Some(&e.md_hash) {
        debug!("unchanged: {table} {id}");
        return Ok(false);
    }

    match &e.fm {
        FrontMatter::Forum(m)   => upsert_forum(tx, m, &md_path, e.mtime, &e.md_hash)?,
        FrontMatter::Channel(m) => upsert_channel(tx, m, &md_path, e.mtime, &e.md_hash)?,
        FrontMatter::Thread(m)  => upsert_thread(tx, m, &e.body, &md_path, e.mtime, &e.md_hash)?,
        FrontMatter::Reply(m)   => upsert_reply(tx, m, &e.body, &md_path, e.mtime, &e.md_hash)?,
    }
    Ok(true)
}

fn upsert_forum(
    tx: &Connection,
    m: &md::ForumMeta,
    md_path: &str,
    mtime: i64,
    md_hash: &str,
) -> rusqlite::Result<()> {
    let visibility = serde_json::to_string(&m.visibility)
        .unwrap_or_else(|_| "\"public\"".into())
        .trim_matches('"')
        .to_string();
    tx.execute(
        "INSERT INTO forum (id, path, name, parent_id, visibility, group_id, created_at, created_by, md_path, md_mtime, md_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(id) DO UPDATE SET
             path=excluded.path, name=excluded.name, parent_id=excluded.parent_id,
             visibility=excluded.visibility, group_id=excluded.group_id,
             created_at=excluded.created_at, created_by=excluded.created_by,
             md_path=excluded.md_path, md_mtime=excluded.md_mtime, md_hash=excluded.md_hash",
        params![
            m.id.to_string(),
            m.path,
            m.name,
            m.parent,
            visibility,
            m.group,
            m.created_at.to_rfc3339(),
            m.created_by,
            md_path,
            mtime,
            md_hash,
        ],
    )?;
    Ok(())
}

fn upsert_channel(
    tx: &Connection,
    m: &md::ChannelMeta,
    md_path: &str,
    mtime: i64,
    md_hash: &str,
) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO channel (id, forum_id, path, name, topic, sort, archived, created_at, created_by, md_path, md_mtime, md_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET
             forum_id=excluded.forum_id, path=excluded.path, name=excluded.name,
             topic=excluded.topic, sort=excluded.sort, archived=excluded.archived,
             created_at=excluded.created_at, created_by=excluded.created_by,
             md_path=excluded.md_path, md_mtime=excluded.md_mtime, md_hash=excluded.md_hash",
        params![
            m.id.to_string(),
            m.forum.to_string(),
            m.path,
            m.name,
            m.topic,
            m.sort,
            m.archived as i32,
            m.created_at.to_rfc3339(),
            m.created_by,
            md_path,
            mtime,
            md_hash,
        ],
    )?;
    Ok(())
}

fn upsert_thread(
    tx: &Connection,
    m: &md::ThreadMeta,
    body: &str,
    md_path: &str,
    mtime: i64,
    md_hash: &str,
) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO thread (id, channel_id, forum_id, title, author, ts, edited_at, pinned, locked, deleted, md_path, md_mtime, md_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(id) DO UPDATE SET
             channel_id=excluded.channel_id, forum_id=excluded.forum_id, title=excluded.title,
             author=excluded.author, ts=excluded.ts, edited_at=excluded.edited_at,
             pinned=excluded.pinned, locked=excluded.locked, deleted=excluded.deleted,
             md_path=excluded.md_path, md_mtime=excluded.md_mtime, md_hash=excluded.md_hash",
        params![
            m.id.to_string(),
            m.channel.to_string(),
            m.forum.to_string(),
            m.title,
            m.author,
            m.ts.to_rfc3339(),
            m.edited_at.map(|t| t.to_rfc3339()),
            m.pinned as i32,
            m.locked as i32,
            m.deleted as i32,
            md_path,
            mtime,
            md_hash,
        ],
    )?;
    // tags
    tx.execute("DELETE FROM thread_tag WHERE thread_id = ?1", params![m.id.to_string()])?;
    for tag in &m.tags {
        tx.execute(
            "INSERT OR IGNORE INTO thread_tag(thread_id, tag) VALUES (?1, ?2)",
            params![m.id.to_string(), tag],
        )?;
    }
    // FTS
    tx.execute(
        "DELETE FROM thread_fts WHERE thread_id = ?1",
        params![m.id.to_string()],
    )?;
    let plain = text::to_plain(body);
    tx.execute(
        "INSERT INTO thread_fts(title, body, thread_id, channel_id) VALUES (?1, ?2, ?3, ?4)",
        params![m.title, plain, m.id.to_string(), m.channel.to_string()],
    )?;
    Ok(())
}

fn upsert_reply(
    tx: &Connection,
    m: &md::ReplyMeta,
    body: &str,
    md_path: &str,
    mtime: i64,
    md_hash: &str,
) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO reply (id, thread_id, parent_id, forum_id, channel_id, author, ts, edited_at, deleted, md_path, md_mtime, md_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET
             thread_id=excluded.thread_id, parent_id=excluded.parent_id,
             forum_id=excluded.forum_id, channel_id=excluded.channel_id,
             author=excluded.author, ts=excluded.ts, edited_at=excluded.edited_at,
             deleted=excluded.deleted,
             md_path=excluded.md_path, md_mtime=excluded.md_mtime, md_hash=excluded.md_hash",
        params![
            m.id.to_string(),
            m.thread.to_string(),
            m.parent.to_string(),
            m.forum.to_string(),
            m.channel.to_string(),
            m.author,
            m.ts.to_rfc3339(),
            m.edited_at.map(|t| t.to_rfc3339()),
            m.deleted as i32,
            md_path,
            mtime,
            md_hash,
        ],
    )?;
    // mentions
    tx.execute("DELETE FROM reply_mention WHERE reply_id = ?1", params![m.id.to_string()])?;
    for u in &m.mentions {
        tx.execute(
            "INSERT OR IGNORE INTO reply_mention(reply_id, user_uri) VALUES (?1, ?2)",
            params![m.id.to_string(), u],
        )?;
    }
    // attachments (再投入)
    tx.execute("DELETE FROM reply_attachment WHERE reply_id = ?1", params![m.id.to_string()])?;
    for (i, a) in m.attachments.iter().enumerate() {
        tx.execute(
            "INSERT INTO reply_attachment(reply_id, seq, kind, cid, name) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![m.id.to_string(), i as i64, a.kind, a.cid, a.name],
        )?;
    }
    // reactions: snapshot replace
    tx.execute("DELETE FROM reply_reaction WHERE reply_id = ?1", params![m.id.to_string()])?;
    let now = chrono::Utc::now().to_rfc3339();
    for (emoji, users) in &m.reactions {
        for u in users {
            tx.execute(
                "INSERT OR IGNORE INTO reply_reaction(reply_id, emoji, user_uri, ts) VALUES (?1, ?2, ?3, ?4)",
                params![m.id.to_string(), emoji, u, now],
            )?;
        }
    }
    // FTS
    tx.execute("DELETE FROM reply_fts WHERE reply_id = ?1", params![m.id.to_string()])?;
    let plain = text::to_plain(body);
    tx.execute(
        "INSERT INTO reply_fts(content, thread_id, reply_id, author, ts) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![plain, m.thread.to_string(), m.id.to_string(), m.author, m.ts.to_rfc3339()],
    )?;
    // thread の last_reply_ts / reply_count を recalc (簡易、 同 transaction で)
    tx.execute(
        "UPDATE thread SET
             last_reply_ts = (SELECT MAX(ts) FROM reply WHERE thread_id = ?1 AND deleted = 0),
             reply_count   = (SELECT COUNT(*) FROM reply WHERE thread_id = ?1 AND deleted = 0)
         WHERE id = ?1",
        params![m.thread.to_string()],
    )?;
    Ok(())
}

/// db には残っているが FS に md が見当たらない行を消す。
/// reindex_all の後に呼ぶ想定。 必要に応じて 1 forum 単位に絞れるよう拡張可能。
pub fn prune_missing(db: &mut Db, store: &MdStore) -> rusqlite::Result<usize> {
    let mut total = 0usize;
    for table in ["forum", "channel", "thread", "reply"] {
        let mut stmt = db.conn.prepare(&format!("SELECT id, md_path FROM {table}"))?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();
        drop(stmt);
        let mut to_delete: Vec<String> = Vec::new();
        for (id, rel) in rows {
            let abs: PathBuf = store.forum_root.join(rel);
            if !abs.exists() {
                to_delete.push(id);
            }
        }
        for id in &to_delete {
            db.conn.execute(&format!("DELETE FROM {table} WHERE id = ?1"), params![id])?;
        }
        total += to_delete.len();
    }
    Ok(total)
}
