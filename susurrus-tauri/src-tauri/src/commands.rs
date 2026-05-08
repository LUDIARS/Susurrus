//! Tauri IPC commands。 すべて &State<AppState> を取って susurrus-core に委譲。

use crate::state::AppState;
use serde::Deserialize;
use susurrus_core::{compose, indexer, presence, query};
use susurrus_md::Visibility;
use tauri::{Manager, State};
use uuid::Uuid;

/// compose 後に Synergos bridge へ md ファイルを publish する。
/// 失敗しても compose 自体は成功扱い (chain への乗せ直しは reindex でも可)。
async fn publish_to_synergos(state: &std::sync::Arc<AppState>, rel_md_path: &str) {
    let abs = {
        let inner = state.inner.lock();
        inner.store.forum_root.join(rel_md_path)
    };
    if let Err(e) = state.synergos.publish(&[abs.as_path()]).await {
        tracing::warn!("synergos publish failed (non-fatal): {e:#}");
    }
}

fn ts(e: impl std::fmt::Display) -> String { e.to_string() }

#[tauri::command]
pub fn ping() -> &'static str { "pong" }

#[tauri::command]
pub fn list_forums(state: State<'_, std::sync::Arc<AppState>>) -> Result<Vec<query::ForumRow>, String> {
    let inner = state.inner.lock();
    query::list_forums(&inner.db.conn).map_err(ts)
}

#[tauri::command]
pub fn list_channels(
    state: State<'_, std::sync::Arc<AppState>>,
    forum_id: String,
) -> Result<Vec<query::ChannelRow>, String> {
    let inner = state.inner.lock();
    query::list_channels(&inner.db.conn, &forum_id).map_err(ts)
}

#[tauri::command]
pub fn list_threads(
    state: State<'_, std::sync::Arc<AppState>>,
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
    state: State<'_, std::sync::Arc<AppState>>,
    thread_id: String,
) -> Result<Vec<query::ReplyRow>, String> {
    let inner = state.inner.lock();
    query::list_replies(&inner.db.conn, &thread_id).map_err(ts)
}

#[tauri::command]
pub fn search_replies(
    state: State<'_, std::sync::Arc<AppState>>,
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
pub async fn create_forum(
    state: State<'_, std::sync::Arc<AppState>>,
    args: CreateForumArgs,
) -> Result<String, String> {
    let (id, rel) = {
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
        (m.id.to_string(), format!("{}/_forum.md", args.path))
    };
    publish_to_synergos(&state, &rel).await;
    Ok(id)
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
pub async fn create_channel(
    state: State<'_, std::sync::Arc<AppState>>,
    args: CreateChannelArgs,
) -> Result<String, String> {
    let forum_id = Uuid::parse_str(&args.forum_id).map_err(ts)?;
    let (id, rel) = {
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
        (m.id.to_string(), format!("{}/{}/_channel.md", args.forum_path, args.name))
    };
    publish_to_synergos(&state, &rel).await;
    Ok(id)
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
pub async fn create_thread(
    state: State<'_, std::sync::Arc<AppState>>,
    args: CreateThreadArgs,
) -> Result<String, String> {
    let forum_id = Uuid::parse_str(&args.forum_id).map_err(ts)?;
    let channel_id = Uuid::parse_str(&args.channel_id).map_err(ts)?;
    let (id, rel) = {
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
        // 直前の DB row から実 md_path を取れるが、 一旦 query で確認
        let md_path: String = inner
            .db
            .conn
            .query_row(
                "SELECT md_path FROM thread WHERE id = ?1",
                [m.id.to_string()],
                |r| r.get(0),
            )
            .map_err(ts)?;
        (m.id.to_string(), md_path)
    };
    publish_to_synergos(&state, &rel).await;
    Ok(id)
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
pub async fn create_reply(
    state: State<'_, std::sync::Arc<AppState>>,
    args: CreateReplyArgs,
) -> Result<String, String> {
    let forum_id = Uuid::parse_str(&args.forum_id).map_err(ts)?;
    let channel_id = Uuid::parse_str(&args.channel_id).map_err(ts)?;
    let thread_id = Uuid::parse_str(&args.thread_id).map_err(ts)?;
    let parent_id = Uuid::parse_str(&args.parent_id).map_err(ts)?;
    let (id, rel) = {
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
        let md_path: String = inner
            .db
            .conn
            .query_row(
                "SELECT md_path FROM reply WHERE id = ?1",
                [m.id.to_string()],
                |r| r.get(0),
            )
            .map_err(ts)?;
        (m.id.to_string(), md_path)
    };
    publish_to_synergos(&state, &rel).await;
    Ok(id)
}

#[tauri::command]
pub fn reindex_all(state: State<'_, std::sync::Arc<AppState>>) -> Result<indexer::IndexStats, String> {
    let mut inner = state.inner.lock();
    let store = inner.store.clone_handle();
    indexer::reindex_all(&mut inner.db, &store).map_err(ts)
}

#[tauri::command]
pub fn read_thread_body(
    state: State<'_, std::sync::Arc<AppState>>,
    thread_id: String,
) -> Result<query::BodyResponse, String> {
    let inner = state.inner.lock();
    query::read_thread_body(&inner.db.conn, &inner.store, &thread_id).map_err(ts)
}

#[tauri::command]
pub fn read_reply_body(
    state: State<'_, std::sync::Arc<AppState>>,
    reply_id: String,
) -> Result<query::BodyResponse, String> {
    let inner = state.inner.lock();
    query::read_reply_body(&inner.db.conn, &inner.store, &reply_id).map_err(ts)
}

#[tauri::command]
pub fn start_typing(
    state: State<'_, std::sync::Arc<AppState>>,
    thread_id: String,
    extend_ms: Option<i64>,
) -> Result<(), String> {
    let thread = Uuid::parse_str(&thread_id).map_err(ts)?;
    let mut inner = state.inner.lock();
    let user = inner.current_user.clone();
    let inner: &mut crate::state::Inner = &mut inner;
    let _ = presence::local_start_typing(
        &inner.db.conn,
        &mut inner.typing,
        thread,
        &user,
        extend_ms.unwrap_or(3_000),
    )
    .map_err(ts)?;
    Ok(())
}

#[tauri::command]
pub fn list_typing(
    state: State<'_, std::sync::Arc<AppState>>,
    thread_id: String,
) -> Result<Vec<presence::TypingRow>, String> {
    let inner = state.inner.lock();
    presence::list_typing(&inner.db.conn, &thread_id).map_err(ts)
}

#[tauri::command]
pub fn current_user(state: State<'_, std::sync::Arc<AppState>>) -> Result<String, String> {
    Ok(state.inner.lock().current_user.clone())
}

// ──────────────────────────────────────────────────────────────────
// Memoria 連携
// ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SaveToMemoriaArgs {
    pub reply_id: String,
    pub thread_title: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[tauri::command]
pub async fn save_to_memoria(
    state: State<'_, std::sync::Arc<AppState>>,
    args: SaveToMemoriaArgs,
) -> Result<String, String> {
    let (body, md_path) = {
        let inner = state.inner.lock();
        let r = susurrus_core::query::read_reply_body(&inner.db.conn, &inner.store, &args.reply_id)
            .map_err(ts)?;
        (r.body, r.md_path)
    };
    let saved = state
        .memoria
        .save_bookmark(&susurrus_memoria::SaveBookmark {
            url: None,
            title: args.thread_title,
            body,
            source: format!("susurrus:reply:{}", args.reply_id),
            tags: args.tags,
            created_at: Some(chrono::Utc::now()),
        })
        .await
        .map_err(ts)?;
    tracing::info!("memoria: saved reply {} from md_path {} as bookmark {}", args.reply_id, md_path, saved.id);
    Ok(saved.id)
}

#[tauri::command]
pub async fn memoria_dig(
    state: State<'_, std::sync::Arc<AppState>>,
    url: String,
) -> Result<susurrus_memoria::DigResult, String> {
    state.memoria.request_dig(&url).await.map_err(ts)
}

#[tauri::command]
pub fn memoria_enabled(state: State<'_, std::sync::Arc<AppState>>) -> Result<bool, String> {
    Ok(state.memoria.enabled)
}

// ──────────────────────────────────────────────────────────────────
// Multi-window (detach)
// ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn open_thread_window(
    app: tauri::AppHandle,
    thread_id: String,
) -> Result<(), String> {
    let label = format!("thread-{}", thread_id.replace('-', "_"));
    if app.get_webview_window(&label).is_some() {
        // 既に開いている → focus
        if let Some(w) = app.get_webview_window(&label) {
            let _ = w.set_focus();
        }
        return Ok(());
    }
    let url = format!("index.html#/thread/{thread_id}");
    let _ = tauri::WebviewWindowBuilder::new(
        &app,
        label,
        tauri::WebviewUrl::App(url.into()),
    )
    .title(format!("Susurrus — {}", thread_id))
    .inner_size(700.0, 800.0)
    .build()
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn close_window(app: tauri::AppHandle, label: String) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(&label) {
        w.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}
