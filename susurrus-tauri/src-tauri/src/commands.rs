//! Tauri IPC commands。 すべて &State<AppState> を取って susurrus-core に委譲。

use crate::state::AppState;
use serde::Deserialize;
use susurrus_core::{compose, indexer, query};
use susurrus_md::Visibility;
use tauri::State;
use uuid::Uuid;

fn ts(e: impl std::fmt::Display) -> String { e.to_string() }

#[tauri::command]
pub fn ping() -> &'static str { "pong" }

#[tauri::command]
pub fn list_forums(state: State<'_, AppState>) -> Result<Vec<query::ForumRow>, String> {
    let inner = state.inner.lock();
    query::list_forums(&inner.db.conn).map_err(ts)
}

#[tauri::command]
pub fn list_channels(
    state: State<'_, AppState>,
    forum_id: String,
) -> Result<Vec<query::ChannelRow>, String> {
    let inner = state.inner.lock();
    query::list_channels(&inner.db.conn, &forum_id).map_err(ts)
}

#[tauri::command]
pub fn list_threads(
    state: State<'_, AppState>,
    channel_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<query::ThreadRow>, String> {
    let inner = state.inner.lock();
    query::list_threads(
        &inner.db.conn,
        &channel_id,
        limit.unwrap_or(50),
        offset.unwrap_or(0),
    )
    .map_err(ts)
}

#[tauri::command]
pub fn list_replies(
    state: State<'_, AppState>,
    thread_id: String,
) -> Result<Vec<query::ReplyRow>, String> {
    let inner = state.inner.lock();
    query::list_replies(&inner.db.conn, &thread_id).map_err(ts)
}

#[tauri::command]
pub fn search_replies(
    state: State<'_, AppState>,
    q: String,
    limit: Option<i64>,
) -> Result<Vec<query::ReplySearchHit>, String> {
    let inner = state.inner.lock();
    query::search_replies(&inner.db.conn, &q, limit.unwrap_or(50)).map_err(ts)
}

#[derive(Deserialize)]
pub struct CreateForumArgs {
    pub path: String,
    pub name: String,
    pub visibility: Visibility,
    pub group: Option<String>,
    pub created_by: String,
}

#[tauri::command]
pub fn create_forum(
    state: State<'_, AppState>,
    args: CreateForumArgs,
) -> Result<String, String> {
    let mut inner = state.inner.lock();
    let store = inner.store.clone_handle();
    let m = compose::create_forum(
        &mut inner.db,
        &store,
        &args.path,
        &args.name,
        args.visibility,
        args.group,
        &args.created_by,
    )
    .map_err(ts)?;
    Ok(m.id.to_string())
}

#[derive(Deserialize)]
pub struct CreateChannelArgs {
    pub forum_id: String,
    pub forum_path: String,
    pub name: String,
    #[serde(default)]
    pub topic: String,
    #[serde(default = "default_sort")]
    pub sort: i32,
    pub created_by: String,
}
fn default_sort() -> i32 { 100 }

#[tauri::command]
pub fn create_channel(
    state: State<'_, AppState>,
    args: CreateChannelArgs,
) -> Result<String, String> {
    let forum_id = Uuid::parse_str(&args.forum_id).map_err(ts)?;
    let mut inner = state.inner.lock();
    let store = inner.store.clone_handle();
    let m = compose::create_channel(
        &mut inner.db,
        &store,
        forum_id,
        &args.forum_path,
        &args.name,
        &args.topic,
        args.sort,
        &args.created_by,
    )
    .map_err(ts)?;
    Ok(m.id.to_string())
}

#[derive(Deserialize)]
pub struct CreateThreadArgs {
    pub forum_id: String,
    pub channel_id: String,
    pub channel_path: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub author: String,
}

#[tauri::command]
pub fn create_thread(
    state: State<'_, AppState>,
    args: CreateThreadArgs,
) -> Result<String, String> {
    let forum_id = Uuid::parse_str(&args.forum_id).map_err(ts)?;
    let channel_id = Uuid::parse_str(&args.channel_id).map_err(ts)?;
    let mut inner = state.inner.lock();
    let store = inner.store.clone_handle();
    let m = compose::create_thread(
        &mut inner.db,
        &store,
        forum_id,
        channel_id,
        &args.channel_path,
        &args.title,
        &args.body,
        args.tags,
        &args.author,
    )
    .map_err(ts)?;
    Ok(m.id.to_string())
}

#[derive(Deserialize)]
pub struct CreateReplyArgs {
    pub forum_id: String,
    pub channel_id: String,
    pub thread_id: String,
    pub thread_md_path: String,
    pub parent_id: String,
    pub body: String,
    pub author: String,
    #[serde(default)]
    pub mentions: Vec<String>,
}

#[tauri::command]
pub fn create_reply(
    state: State<'_, AppState>,
    args: CreateReplyArgs,
) -> Result<String, String> {
    let forum_id = Uuid::parse_str(&args.forum_id).map_err(ts)?;
    let channel_id = Uuid::parse_str(&args.channel_id).map_err(ts)?;
    let thread_id = Uuid::parse_str(&args.thread_id).map_err(ts)?;
    let parent_id = Uuid::parse_str(&args.parent_id).map_err(ts)?;
    let mut inner = state.inner.lock();
    let store = inner.store.clone_handle();
    let m = compose::create_reply(
        &mut inner.db,
        &store,
        forum_id,
        channel_id,
        thread_id,
        &args.thread_md_path,
        parent_id,
        &args.body,
        &args.author,
        args.mentions,
        Vec::new(),
    )
    .map_err(ts)?;
    Ok(m.id.to_string())
}

#[tauri::command]
pub fn reindex_all(state: State<'_, AppState>) -> Result<indexer::IndexStats, String> {
    let mut inner = state.inner.lock();
    let store = inner.store.clone_handle();
    indexer::reindex_all(&mut inner.db, &store).map_err(ts)
}

#[tauri::command]
pub fn read_thread_body(
    state: State<'_, AppState>,
    thread_id: String,
) -> Result<query::BodyResponse, String> {
    let inner = state.inner.lock();
    query::read_thread_body(&inner.db.conn, &inner.store, &thread_id).map_err(ts)
}

#[tauri::command]
pub fn read_reply_body(
    state: State<'_, AppState>,
    reply_id: String,
) -> Result<query::BodyResponse, String> {
    let inner = state.inner.lock();
    query::read_reply_body(&inner.db.conn, &inner.store, &reply_id).map_err(ts)
}
