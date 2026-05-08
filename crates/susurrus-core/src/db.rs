//! SQLite キャッシュ。 完全に再生成可能。
//! 仕様: ../../../spec/DB-SCHEMA.md

use rusqlite::Connection;
use std::path::Path;

pub struct Db {
    pub conn: Connection,
}

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous  = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA temp_store   = MEMORY;
             PRAGMA mmap_size    = 268435456;",
        )?;
        let mut db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&mut self) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute_batch(SCHEMA_V1)?;
        tx.execute(
            "INSERT OR IGNORE INTO susurrus_meta(key, value) VALUES('schema_version', '1')",
            [],
        )?;
        tx.commit()?;
        Ok(())
    }
}

const SCHEMA_V1: &str = r#"
CREATE TABLE IF NOT EXISTS susurrus_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS forum (
    id          TEXT PRIMARY KEY,
    path        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    parent_id   TEXT,
    visibility  TEXT NOT NULL,
    group_id    TEXT,
    created_at  TEXT NOT NULL,
    created_by  TEXT NOT NULL,
    md_path     TEXT NOT NULL,
    md_mtime    INTEGER NOT NULL,
    md_hash     TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_forum_parent ON forum(parent_id);
CREATE INDEX IF NOT EXISTS idx_forum_path   ON forum(path);

CREATE TABLE IF NOT EXISTS channel (
    id          TEXT PRIMARY KEY,
    forum_id    TEXT NOT NULL REFERENCES forum(id) ON DELETE CASCADE,
    path        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    topic       TEXT NOT NULL DEFAULT '',
    sort        INTEGER NOT NULL DEFAULT 100,
    archived    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    created_by  TEXT NOT NULL,
    md_path     TEXT NOT NULL,
    md_mtime    INTEGER NOT NULL,
    md_hash     TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_channel_forum ON channel(forum_id, sort);

CREATE TABLE IF NOT EXISTS thread (
    id              TEXT PRIMARY KEY,
    channel_id      TEXT NOT NULL REFERENCES channel(id) ON DELETE CASCADE,
    forum_id        TEXT NOT NULL,
    title           TEXT NOT NULL,
    author          TEXT NOT NULL,
    ts              TEXT NOT NULL,
    edited_at       TEXT,
    pinned          INTEGER NOT NULL DEFAULT 0,
    locked          INTEGER NOT NULL DEFAULT 0,
    deleted         INTEGER NOT NULL DEFAULT 0,
    last_reply_ts   TEXT,
    reply_count     INTEGER NOT NULL DEFAULT 0,
    md_path         TEXT NOT NULL,
    md_mtime        INTEGER NOT NULL,
    md_hash         TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_thread_channel_lastreply ON thread(channel_id, last_reply_ts DESC);
CREATE INDEX IF NOT EXISTS idx_thread_pinned            ON thread(channel_id, pinned DESC, last_reply_ts DESC);
CREATE INDEX IF NOT EXISTS idx_thread_author            ON thread(author, ts DESC);

CREATE TABLE IF NOT EXISTS thread_tag (
    thread_id  TEXT NOT NULL REFERENCES thread(id) ON DELETE CASCADE,
    tag        TEXT NOT NULL,
    PRIMARY KEY(thread_id, tag)
);
CREATE INDEX IF NOT EXISTS idx_thread_tag_tag ON thread_tag(tag);

CREATE TABLE IF NOT EXISTS reply (
    id          TEXT PRIMARY KEY,
    thread_id   TEXT NOT NULL REFERENCES thread(id) ON DELETE CASCADE,
    parent_id   TEXT NOT NULL,
    forum_id    TEXT NOT NULL,
    channel_id  TEXT NOT NULL,
    author      TEXT NOT NULL,
    ts          TEXT NOT NULL,
    edited_at   TEXT,
    deleted     INTEGER NOT NULL DEFAULT 0,
    md_path     TEXT NOT NULL,
    md_mtime    INTEGER NOT NULL,
    md_hash     TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_reply_thread_ts  ON reply(thread_id, ts);
CREATE INDEX IF NOT EXISTS idx_reply_parent     ON reply(parent_id);
CREATE INDEX IF NOT EXISTS idx_reply_author_ts  ON reply(author, ts DESC);

CREATE TABLE IF NOT EXISTS reply_mention (
    reply_id  TEXT NOT NULL REFERENCES reply(id) ON DELETE CASCADE,
    user_uri  TEXT NOT NULL,
    PRIMARY KEY(reply_id, user_uri)
);
CREATE INDEX IF NOT EXISTS idx_mention_user ON reply_mention(user_uri);

CREATE TABLE IF NOT EXISTS reply_attachment (
    reply_id  TEXT NOT NULL REFERENCES reply(id) ON DELETE CASCADE,
    seq       INTEGER NOT NULL,
    kind      TEXT NOT NULL,
    cid       TEXT NOT NULL,
    name      TEXT NOT NULL,
    PRIMARY KEY(reply_id, seq)
);

CREATE TABLE IF NOT EXISTS reply_reaction (
    reply_id  TEXT NOT NULL REFERENCES reply(id) ON DELETE CASCADE,
    emoji     TEXT NOT NULL,
    user_uri  TEXT NOT NULL,
    ts        TEXT NOT NULL,
    PRIMARY KEY(reply_id, emoji, user_uri)
);
CREATE INDEX IF NOT EXISTS idx_reaction_reply ON reply_reaction(reply_id);

CREATE VIRTUAL TABLE IF NOT EXISTS reply_fts USING fts5(
    content,
    thread_id UNINDEXED,
    reply_id  UNINDEXED,
    author    UNINDEXED,
    ts        UNINDEXED,
    tokenize = 'trigram'
);

CREATE VIRTUAL TABLE IF NOT EXISTS thread_fts USING fts5(
    title,
    body,
    thread_id  UNINDEXED,
    channel_id UNINDEXED,
    tokenize = 'trigram'
);

CREATE TABLE IF NOT EXISTS presence (
    user_uri    TEXT PRIMARY KEY,
    peer_id     TEXT,
    state       TEXT NOT NULL,
    last_seen   TEXT NOT NULL,
    transport   TEXT NOT NULL DEFAULT '',
    rtt_ms      INTEGER,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS typing (
    thread_id  TEXT NOT NULL,
    user_uri   TEXT NOT NULL,
    until      TEXT NOT NULL,
    PRIMARY KEY(thread_id, user_uri)
);
CREATE INDEX IF NOT EXISTS idx_typing_until ON typing(until);

CREATE TABLE IF NOT EXISTS read_cursor (
    user_uri    TEXT NOT NULL,
    thread_id   TEXT NOT NULL,
    last_read_reply_id TEXT,
    last_read_ts       TEXT,
    PRIMARY KEY(user_uri, thread_id)
);

CREATE TABLE IF NOT EXISTS peer (
    peer_id     TEXT PRIMARY KEY,
    user_uri    TEXT NOT NULL,
    label       TEXT NOT NULL DEFAULT '',
    first_seen  TEXT NOT NULL,
    last_seen   TEXT
);
CREATE INDEX IF NOT EXISTS idx_peer_user ON peer(user_uri);

CREATE TABLE IF NOT EXISTS forum_subscription (
    forum_id  TEXT NOT NULL REFERENCES forum(id) ON DELETE CASCADE,
    peer_id   TEXT NOT NULL,
    PRIMARY KEY(forum_id, peer_id)
);

CREATE TABLE IF NOT EXISTS setting (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL
);
"#;
