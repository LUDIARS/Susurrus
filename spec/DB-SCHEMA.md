# Susurrus SQLite キャッシュ DB スキーマ

> `$SUSURRUS_DATA/db/susurrus.db`。 **完全に再生成可能** な derived state のみを置く。 正本は md ファイル (= MD-SCHEMA.md)。 破損したら削除して再 index すれば良い。

## 0. 設計原則

- 正本は md。 SQLite は **read 高速化** + **realtime presence/typing** + **FTS** のためだけに存在
- スキーマの breaking change は migration を書かず、 ファイル削除 → 再 index でリセット
- 個人データ (名前 / avatar) は持たない (Cernere に都度問い合わせ)
- すべての文字列カラムは `TEXT NOT NULL DEFAULT ''` を基本とする (NULL を最小化)
- 時刻は ISO-8601 文字列で持つ (検索容易性 > サイズ)

## 1. version / meta

```sql
CREATE TABLE susurrus_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
-- 例:
-- ('schema_version', '1')
-- ('last_full_reindex', '2026-05-08T12:00:00+09:00')
-- ('cernere_token_user_uri', 'cr:...')
```

## 2. forum / channel / thread / reply (md キャッシュ)

### 2.1 forum

```sql
CREATE TABLE forum (
    id          TEXT PRIMARY KEY,            -- f_*
    path        TEXT NOT NULL UNIQUE,        -- forum_root からの相対 path
    name        TEXT NOT NULL,
    parent_id   TEXT,                        -- 親 forum id (NULL = root)
    visibility  TEXT NOT NULL,               -- public | cernere-group | invite-only | dm
    group_id    TEXT,                        -- cernere-group のとき
    created_at  TEXT NOT NULL,
    created_by  TEXT NOT NULL,               -- cr:<uuid>
    md_path     TEXT NOT NULL,               -- 実体 md path (= path/_forum.md)
    md_mtime    INTEGER NOT NULL,            -- unix epoch ms
    md_hash     TEXT NOT NULL                -- blake3 of md content
);
CREATE INDEX idx_forum_parent  ON forum(parent_id);
CREATE INDEX idx_forum_path    ON forum(path);
```

### 2.2 channel

```sql
CREATE TABLE channel (
    id          TEXT PRIMARY KEY,            -- c_*
    forum_id    TEXT NOT NULL REFERENCES forum(id) ON DELETE CASCADE,
    path        TEXT NOT NULL UNIQUE,        -- forum_root からの相対 path
    name        TEXT NOT NULL,
    topic       TEXT NOT NULL DEFAULT '',
    sort        INTEGER NOT NULL DEFAULT 100,
    archived    INTEGER NOT NULL DEFAULT 0,  -- 0/1
    created_at  TEXT NOT NULL,
    created_by  TEXT NOT NULL,
    md_path     TEXT NOT NULL,
    md_mtime    INTEGER NOT NULL,
    md_hash     TEXT NOT NULL
);
CREATE INDEX idx_channel_forum  ON channel(forum_id, sort);
```

### 2.3 thread

```sql
CREATE TABLE thread (
    id              TEXT PRIMARY KEY,        -- t_*
    channel_id      TEXT NOT NULL REFERENCES channel(id) ON DELETE CASCADE,
    forum_id        TEXT NOT NULL,           -- denormalize for fast lookup
    title           TEXT NOT NULL,
    author          TEXT NOT NULL,           -- cr:<uuid>
    ts              TEXT NOT NULL,           -- created_at
    edited_at       TEXT,
    pinned          INTEGER NOT NULL DEFAULT 0,
    locked          INTEGER NOT NULL DEFAULT 0,
    deleted         INTEGER NOT NULL DEFAULT 0,
    last_reply_ts   TEXT,                    -- thread のソートに使用
    reply_count     INTEGER NOT NULL DEFAULT 0,
    md_path         TEXT NOT NULL,
    md_mtime        INTEGER NOT NULL,
    md_hash         TEXT NOT NULL
);
CREATE INDEX idx_thread_channel_lastreply ON thread(channel_id, last_reply_ts DESC);
CREATE INDEX idx_thread_pinned            ON thread(channel_id, pinned DESC, last_reply_ts DESC);
CREATE INDEX idx_thread_author            ON thread(author, ts DESC);

CREATE TABLE thread_tag (
    thread_id  TEXT NOT NULL REFERENCES thread(id) ON DELETE CASCADE,
    tag        TEXT NOT NULL,
    PRIMARY KEY(thread_id, tag)
);
CREATE INDEX idx_thread_tag_tag ON thread_tag(tag);
```

### 2.4 reply

```sql
CREATE TABLE reply (
    id          TEXT PRIMARY KEY,            -- m_*
    thread_id   TEXT NOT NULL REFERENCES thread(id) ON DELETE CASCADE,
    parent_id   TEXT NOT NULL,               -- thread root id か別 reply id
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
CREATE INDEX idx_reply_thread_ts   ON reply(thread_id, ts);
CREATE INDEX idx_reply_parent      ON reply(parent_id);
CREATE INDEX idx_reply_author_ts   ON reply(author, ts DESC);

CREATE TABLE reply_mention (
    reply_id  TEXT NOT NULL REFERENCES reply(id) ON DELETE CASCADE,
    user_uri  TEXT NOT NULL,
    PRIMARY KEY(reply_id, user_uri)
);
CREATE INDEX idx_mention_user ON reply_mention(user_uri);

CREATE TABLE reply_attachment (
    reply_id  TEXT NOT NULL REFERENCES reply(id) ON DELETE CASCADE,
    seq       INTEGER NOT NULL,
    kind      TEXT NOT NULL,                 -- image | video | binary
    cid       TEXT NOT NULL,                 -- blake3 of content (Synergos CID)
    name      TEXT NOT NULL,
    PRIMARY KEY(reply_id, seq)
);

CREATE TABLE reply_reaction (
    reply_id  TEXT NOT NULL REFERENCES reply(id) ON DELETE CASCADE,
    emoji     TEXT NOT NULL,
    user_uri  TEXT NOT NULL,
    ts        TEXT NOT NULL,
    PRIMARY KEY(reply_id, emoji, user_uri)
);
CREATE INDEX idx_reaction_reply ON reply_reaction(reply_id);
```

## 3. 全文検索 (FTS5)

```sql
CREATE VIRTUAL TABLE reply_fts USING fts5(
    content,
    thread_id UNINDEXED,
    reply_id  UNINDEXED,
    author    UNINDEXED,
    ts        UNINDEXED,
    tokenize = 'porter unicode61'
);

CREATE VIRTUAL TABLE thread_fts USING fts5(
    title,
    body,
    thread_id UNINDEXED,
    channel_id UNINDEXED,
    tokenize = 'porter unicode61'
);
```

reply / thread の本文を保存しないのは「正本は md ファイル」 ポリシーに従うため。 FTS の `content` カラムだけは検索のために本文を持つ (ただし MATCH 用 derive、 トリガで md と同期)。

## 4. presence / typing / 未読

ここは ephemeral state。 再起動で消えても問題ない。 `PRAGMA journal_mode=WAL` 前提で別 DB ファイルにしても良い (実装時判断)。

```sql
CREATE TABLE presence (
    user_uri    TEXT PRIMARY KEY,           -- cr:<uuid>
    peer_id     TEXT,                       -- Synergos PeerId (現在 active な)
    state       TEXT NOT NULL,              -- active | idle | offline
    last_seen   TEXT NOT NULL,              -- ISO-8601
    transport   TEXT NOT NULL DEFAULT '',   -- quic | webrtc | sleep
    rtt_ms      INTEGER,                    -- 計測値 (active のみ)
    updated_at  TEXT NOT NULL
);

CREATE TABLE typing (
    thread_id  TEXT NOT NULL,
    user_uri   TEXT NOT NULL,
    until      TEXT NOT NULL,               -- いつまで「入力中」を表示するか
    PRIMARY KEY(thread_id, user_uri)
);
CREATE INDEX idx_typing_until ON typing(until);
```

## 5. 既読 cursor

```sql
CREATE TABLE read_cursor (
    user_uri    TEXT NOT NULL,              -- 自分自身 (= 1 PC = 1 row?) ※後述
    thread_id   TEXT NOT NULL,
    last_read_reply_id TEXT,                -- 最後に読んだ reply id (NULL = thread root のみ既読)
    last_read_ts       TEXT,                -- 並び替え用
    PRIMARY KEY(user_uri, thread_id)
);
```

- マルチデバイス時は **per-peer cursor** にしたいので、 後で `peer_id` を PK に追加する余地あり。 v0.1 では 1 user 1 cursor で実装。
- cursor は Synergos chain にも commit して他デバイスへ同期する (= 自分用の小さな chain entry)。

## 6. peer / sync 状態

```sql
CREATE TABLE peer (
    peer_id     TEXT PRIMARY KEY,           -- Synergos PeerId
    user_uri    TEXT NOT NULL,              -- cr:<uuid>
    label       TEXT NOT NULL DEFAULT '',
    first_seen  TEXT NOT NULL,
    last_seen   TEXT
);
CREATE INDEX idx_peer_user ON peer(user_uri);

CREATE TABLE forum_subscription (
    forum_id  TEXT NOT NULL REFERENCES forum(id) ON DELETE CASCADE,
    peer_id   TEXT NOT NULL,                -- Synergos peer
    PRIMARY KEY(forum_id, peer_id)
);
```

## 7. 設定 / opt-out

```sql
CREATE TABLE setting (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL
);
-- 例:
-- ('memoria.enabled', 'true')
-- ('memoria.endpoint', 'http://127.0.0.1:5180')
-- ('typing.send', 'true')
-- ('overlay_sdk.enabled', 'true')
-- ('spatial_chat.enabled', 'false')
```

## 8. md ↔ DB 同期戦略

1. 起動時: forum_root を walk → 各 md を `md_hash` (blake3) で照合 → 差分のみ再 index
2. 動作中: notify (FS watcher) で md 変更を検出 → 同 file path の row を update
3. msg 送信時: core が md を **書く** → notify hook 経由で再 index → SQLite update
4. 受信時 (Synergos auto-pull): md を `forums/**` に書き出し → 同上

整合性チェック (起動時の full reindex) は `last_full_reindex` を見て週次以上 + 明示要求時のみ。

## 9. PRAGMA / 運用

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous  = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA temp_store   = MEMORY;
PRAGMA mmap_size    = 268435456;     -- 256 MiB
```

- バックアップ不要。 削除 → 再 index で復元可能。
- 開発中は `cargo run -- reindex --full` を提供。
