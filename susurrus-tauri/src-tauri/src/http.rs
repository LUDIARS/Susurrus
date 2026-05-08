//! Loopback HTTP server (port 17370 既定)。 susurrus-sdk からの接続を受ける。
//!
//! 認証は v0.0 では無い (loopback 限定 + Cernere 統合は後で)。
//! 公開 endpoint:
//! - GET  /v1/ping
//! - GET  /v1/threads/:thread_id/replies
//! - POST /v1/threads/:thread_id/replies   { author, body }
//! - POST /v1/threads/:thread_id/typing    { user }
//! - POST /v1/spatial/position             { user, forum_id, position }

use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct HttpState {
    pub app: Arc<AppState>,
}

pub fn router(app: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/ping", get(ping))
        .route("/v1/threads/:thread_id/replies", get(list_replies).post(post_reply))
        .route("/v1/threads/:thread_id/typing", post(post_typing))
        .route("/v1/spatial/position", post(post_position))
        .layer(CorsLayer::permissive())
        .with_state(HttpState { app })
}

pub async fn serve(app: Arc<AppState>, port: u16) -> anyhow::Result<()> {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse()?;
    tracing::info!("susurrus http: listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router(app)).await?;
    Ok(())
}

async fn ping() -> &'static str { "pong" }

#[derive(Serialize)]
struct ReplyView {
    id: String,
    author: String,
    ts: String,
    body: String,
}

async fn list_replies(
    State(s): State<HttpState>,
    Path(thread_id): Path<String>,
) -> Result<Json<Vec<ReplyView>>, AppError> {
    let inner = s.app.inner.lock();
    let replies = susurrus_core::query::list_replies(&inner.db.conn, &thread_id)
        .map_err(|e| AppError::sql(e))?;
    let mut views = Vec::with_capacity(replies.len());
    for r in replies {
        let body = susurrus_core::query::read_reply_body(&inner.db.conn, &inner.store, &r.id)
            .map(|b| b.body)
            .unwrap_or_default();
        views.push(ReplyView { id: r.id, author: r.author, ts: r.ts, body });
    }
    Ok(Json(views))
}

#[derive(Deserialize)]
struct PostReplyBody { author: String, body: String }

#[derive(Serialize)]
struct PostReplyResp { id: String }

async fn post_reply(
    State(s): State<HttpState>,
    Path(thread_id): Path<String>,
    Json(body): Json<PostReplyBody>,
) -> Result<Json<PostReplyResp>, AppError> {
    let thread_uuid = uuid::Uuid::parse_str(&thread_id)
        .map_err(|e| AppError::bad_request(format!("invalid thread_id: {e}")))?;
    let (forum_id, channel_id, thread_md_path) = {
        let inner = s.app.inner.lock();
        let row: (String, String, String) = inner.db.conn.query_row(
            "SELECT forum_id, channel_id, md_path FROM thread WHERE id = ?1",
            [&thread_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).map_err(AppError::sql)?;
        row
    };
    let forum_uuid = uuid::Uuid::parse_str(&forum_id).map_err(|e| AppError::sql(rusqlite_other(e)))?;
    let channel_uuid = uuid::Uuid::parse_str(&channel_id).map_err(|e| AppError::sql(rusqlite_other(e)))?;

    let id = {
        let mut inner = s.app.inner.lock();
        let store = inner.store.clone_handle();
        let m = susurrus_core::compose::create_reply(
            &mut inner.db,
            &store,
            forum_uuid,
            channel_uuid,
            thread_uuid,
            &thread_md_path,
            thread_uuid, // root への返信
            &body.body,
            &body.author,
            Vec::new(),
            Vec::new(),
        ).map_err(|e| AppError::internal(format!("compose: {e}")))?;
        m.id.to_string()
    };

    // Synergos へ非同期 publish (失敗は警告のみ)
    let app = s.app.clone();
    let id_clone = id.clone();
    tokio::spawn(async move {
        let abs = {
            let inner = app.inner.lock();
            let md_path: Option<String> = inner.db.conn.query_row(
                "SELECT md_path FROM reply WHERE id = ?1",
                [&id_clone],
                |r| r.get(0),
            ).ok();
            md_path.map(|p| inner.store.forum_root.join(p))
        };
        if let Some(abs) = abs {
            let _ = app.synergos.publish(&[abs.as_path()]).await;
        }
    });

    Ok(Json(PostReplyResp { id }))
}

#[derive(Deserialize)]
struct PostTypingBody { user: String }

async fn post_typing(
    State(s): State<HttpState>,
    Path(thread_id): Path<String>,
    Json(b): Json<PostTypingBody>,
) -> Result<&'static str, AppError> {
    let thread = uuid::Uuid::parse_str(&thread_id)
        .map_err(|e| AppError::bad_request(format!("thread_id: {e}")))?;
    let mut inner = s.app.inner.lock();
    let inner: &mut crate::state::Inner = &mut inner;
    susurrus_core::presence::local_start_typing(
        &inner.db.conn,
        &mut inner.typing,
        thread,
        &b.user,
        3_000,
    ).map_err(AppError::sql)?;
    Ok("ok")
}

#[derive(Deserialize)]
struct PostPositionBody {
    user: String,
    forum_id: String,
    position: serde_json::Value,
}

async fn post_position(
    State(_s): State<HttpState>,
    Json(b): Json<PostPositionBody>,
) -> Result<&'static str, AppError> {
    // 現状はロギングのみ (v1.0 で broadcast へ)
    tracing::info!("spatial position: user={} forum={} pos={}", b.user, b.forum_id, b.position);
    Ok("ok")
}

// ──────────────────────────────────────────────────────────────────
// Error wrapper
// ──────────────────────────────────────────────────────────────────

pub struct AppError {
    status: StatusCode,
    msg: String,
}

impl AppError {
    fn bad_request(m: impl Into<String>) -> Self { Self { status: StatusCode::BAD_REQUEST, msg: m.into() } }
    fn internal(m: impl Into<String>) -> Self { Self { status: StatusCode::INTERNAL_SERVER_ERROR, msg: m.into() } }
    fn sql(e: rusqlite::Error) -> Self { Self::internal(format!("sql: {e}")) }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (self.status, self.msg).into_response()
    }
}

fn rusqlite_other(e: impl std::fmt::Display) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("{e}"),
    )))
}
