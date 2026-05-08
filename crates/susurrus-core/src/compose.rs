//! md ファイル生成 (write 側)。 forum/channel/thread/reply を作る高レベル API。
//!
//! いずれの関数も:
//! 1. UUIDv7 を新規発行
//! 2. md を組み立てて FS に書く
//! 3. indexer::reindex_path で SQLite キャッシュへ反映
//!
//! 生成 path は MD-SCHEMA.md に従う:
//! - forum:   `<forum.path>/_forum.md`
//! - channel: `<channel.path>/_channel.md`
//! - thread:  `<channel.path>/t_<yyyy-mm-dd>_<short>.md`
//! - reply:   `<channel.path>/t_<yyyy-mm-dd>_<short>/m_<short>.md`

use crate::db::Db;
use crate::indexer;
use crate::store::MdStore;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use susurrus_md::{
    self as md, Attachment, ChannelMeta, ForumMeta, FrontMatter, ReplyMeta, ThreadMeta, Visibility,
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ComposeError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("md: {0}")]
    Md(#[from] md::MdError),
    #[error("anyhow: {0}")]
    Other(#[from] anyhow::Error),
}

fn short_id(id: &Uuid) -> String {
    let s = id.simple().to_string();
    s[s.len().saturating_sub(6)..].to_string()
}

fn now() -> DateTime<chrono::FixedOffset> {
    let utc = Utc::now();
    utc.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
}

pub fn create_forum(
    db: &mut Db,
    store: &MdStore,
    path: &str,
    name: &str,
    visibility: Visibility,
    group: Option<String>,
    created_by: &str,
) -> Result<ForumMeta, ComposeError> {
    let id = Uuid::now_v7();
    let parent = path.rsplit_once('/').map(|(p, _)| p.to_string());
    let meta = ForumMeta {
        id,
        path: path.to_string(),
        name: name.to_string(),
        parent,
        visibility,
        group,
        created_at: now(),
        created_by: created_by.to_string(),
    };
    let fm = FrontMatter::Forum(meta.clone());
    let s = md::serialize(&fm, "")?;
    let abs: PathBuf = store.forum_root.join(path).join("_forum.md");
    if let Some(p) = abs.parent() { std::fs::create_dir_all(p)?; }
    std::fs::write(&abs, s)?;
    indexer::reindex_path(db, store, &abs)?;
    Ok(meta)
}

pub fn create_channel(
    db: &mut Db,
    store: &MdStore,
    forum: Uuid,
    forum_path: &str,
    name: &str,
    topic: &str,
    sort: i32,
    created_by: &str,
) -> Result<ChannelMeta, ComposeError> {
    let id = Uuid::now_v7();
    let path = format!("{forum_path}/{name}");
    let meta = ChannelMeta {
        id,
        forum,
        path: path.clone(),
        name: name.to_string(),
        topic: topic.to_string(),
        sort,
        created_at: now(),
        created_by: created_by.to_string(),
        archived: false,
    };
    let fm = FrontMatter::Channel(meta.clone());
    let s = md::serialize(&fm, "")?;
    let abs: PathBuf = store.forum_root.join(&path).join("_channel.md");
    if let Some(p) = abs.parent() { std::fs::create_dir_all(p)?; }
    std::fs::write(&abs, s)?;
    indexer::reindex_path(db, store, &abs)?;
    Ok(meta)
}

pub fn create_thread(
    db: &mut Db,
    store: &MdStore,
    forum: Uuid,
    channel: Uuid,
    channel_path: &str,
    title: &str,
    body: &str,
    tags: Vec<String>,
    author: &str,
) -> Result<ThreadMeta, ComposeError> {
    let id = Uuid::now_v7();
    let ts = now();
    let date = ts.format("%Y-%m-%d").to_string();
    let short = short_id(&id);
    let meta = ThreadMeta {
        id,
        channel,
        forum,
        title: title.to_string(),
        tags,
        author: author.to_string(),
        ts,
        edited_at: None,
        pinned: false,
        locked: false,
        deleted: false,
    };
    let fm = FrontMatter::Thread(meta.clone());
    let s = md::serialize(&fm, body)?;
    let abs: PathBuf = store
        .forum_root
        .join(channel_path)
        .join(format!("t_{date}_{short}.md"));
    if let Some(p) = abs.parent() { std::fs::create_dir_all(p)?; }
    std::fs::write(&abs, s)?;
    indexer::reindex_path(db, store, &abs)?;
    Ok(meta)
}

#[allow(clippy::too_many_arguments)]
pub fn create_reply(
    db: &mut Db,
    store: &MdStore,
    forum: Uuid,
    channel: Uuid,
    thread: Uuid,
    thread_md_path: &str,
    parent: Uuid,
    body: &str,
    author: &str,
    mentions: Vec<String>,
    attachments: Vec<Attachment>,
) -> Result<ReplyMeta, ComposeError> {
    let id = Uuid::now_v7();
    let ts = now();
    let short = short_id(&id);
    let meta = ReplyMeta {
        id,
        thread,
        parent,
        forum,
        channel,
        author: author.to_string(),
        ts,
        edited_at: None,
        deleted: false,
        attachments,
        mentions,
        reactions: std::collections::BTreeMap::new(),
    };
    let fm = FrontMatter::Reply(meta.clone());
    let s = md::serialize(&fm, body)?;
    // thread_md_path = forum_root 相対の thread root md (例 work/ludiars/general/t_*.md)
    // → reply は同名サブディレクトリの中に置く
    let thread_dir = thread_md_path.trim_end_matches(".md");
    let abs: PathBuf = store.forum_root.join(thread_dir).join(format!("m_{short}.md"));
    if let Some(p) = abs.parent() { std::fs::create_dir_all(p)?; }
    std::fs::write(&abs, s)?;
    indexer::reindex_path(db, store, &abs)?;
    Ok(meta)
}
