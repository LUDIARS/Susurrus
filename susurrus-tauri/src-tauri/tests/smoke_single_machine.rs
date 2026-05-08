//! 単 1 台 (Synergos = Noop) で UI/forum/posting が動くかの smoke test。
//!
//! HTTP server 経由 ではなく susurrus-core の compose / query を直接呼ぶ。
//! さらに http::router を `tower::ServiceExt::oneshot` で叩いて HTTP 経路も検証。
//!
//! このテストが pass する = "1 台インストールで投稿できる" という宣伝が嘘でないこと
//! の保証。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::sync::Arc;
use susurrus_core::{compose, query};
use susurrus_md::Visibility;
use susurrus_tauri_lib::{http, state::AppState};
use tower::ServiceExt;

fn boot_state() -> (Arc<AppState>, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    // SUSURRUS_DATA を tmp 下に向ける + Synergos / Memoria は default off
    std::env::remove_var("SUSURRUS_SYNERGOS");
    std::env::set_var("SUSURRUS_MEMORIA_DISABLED", "1");
    let app = AppState::open(tmp.path()).expect("AppState::open");
    (Arc::new(app), tmp)
}

#[test]
fn forum_channel_thread_reply_via_compose() {
    let (state, _tmp) = boot_state();
    let user = "cr:smoketest";

    // forum
    let forum = {
        let mut inner = state.inner.lock();
        let store = inner.store.clone_handle();
        compose::create_forum(
            &mut inner.db,
            &store,
            "smoke/lab",
            "Smoke Lab",
            Visibility::Public,
            None,
            user,
        )
        .unwrap()
    };

    // channel
    let channel = {
        let mut inner = state.inner.lock();
        let store = inner.store.clone_handle();
        compose::create_channel(
            &mut inner.db,
            &store,
            forum.id,
            &forum.path,
            "general",
            "smoke",
            100,
            user,
        )
        .unwrap()
    };

    // thread
    let thread = {
        let mut inner = state.inner.lock();
        let store = inner.store.clone_handle();
        compose::create_thread(
            &mut inner.db,
            &store,
            forum.id,
            channel.id,
            &channel.path,
            "Smoke 投稿",
            "本文 text",
            vec!["smoke".into()],
            user,
        )
        .unwrap()
    };

    // reply
    let thread_md_path: String = {
        let inner = state.inner.lock();
        inner
            .db
            .conn
            .query_row(
                "SELECT md_path FROM thread WHERE id = ?1",
                [thread.id.to_string()],
                |r| r.get(0),
            )
            .unwrap()
    };
    let reply = {
        let mut inner = state.inner.lock();
        let store = inner.store.clone_handle();
        compose::create_reply(
            &mut inner.db,
            &store,
            forum.id,
            channel.id,
            thread.id,
            &thread_md_path,
            thread.id,
            "返信本文",
            "cr:smoke-other",
            vec![user.to_string()],
            Vec::new(),
        )
        .unwrap()
    };

    // query で読み返し
    let inner = state.inner.lock();
    let forums = query::list_forums(&inner.db.conn).unwrap();
    assert_eq!(forums.len(), 1);
    let channels = query::list_channels(&inner.db.conn, &forum.id.to_string()).unwrap();
    assert_eq!(channels.len(), 1);
    let threads = query::list_threads(&inner.db.conn, &channel.id.to_string(), 50, 0).unwrap();
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].reply_count, 1);
    let replies = query::list_replies(&inner.db.conn, &thread.id.to_string()).unwrap();
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].id, reply.id.to_string());

    // body 読み取り (md ファイルから)
    let rb = query::read_reply_body(&inner.db.conn, &inner.store, &reply.id.to_string()).unwrap();
    assert_eq!(rb.body.trim(), "返信本文");
}

#[tokio::test]
async fn http_api_ping_and_replies() {
    let (state, _tmp) = boot_state();

    // データを 1 件用意 (forum + channel + thread)
    let user = "cr:http-smoke";
    let (forum_id, channel_id, thread_id) = {
        let mut inner = state.inner.lock();
        let store = inner.store.clone_handle();
        let f = compose::create_forum(
            &mut inner.db,
            &store,
            "http/smoke",
            "HTTP Smoke",
            Visibility::Public,
            None,
            user,
        )
        .unwrap();
        let c = compose::create_channel(
            &mut inner.db,
            &store,
            f.id,
            &f.path,
            "general",
            "",
            100,
            user,
        )
        .unwrap();
        let t = compose::create_thread(
            &mut inner.db,
            &store,
            f.id,
            c.id,
            &c.path,
            "HTTP Smoke スレッド",
            "thread body",
            vec![],
            user,
        )
        .unwrap();
        (f.id, c.id, t.id)
    };
    let _ = (forum_id, channel_id);

    let app = http::router(state.clone());

    // /v1/ping
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/ping")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"pong");

    // /v1/threads/<id>/replies (空)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/threads/{}/replies", thread_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v.is_array(), "list_replies should return array; got {v}");
    assert_eq!(v.as_array().unwrap().len(), 0);

    // POST reply
    let post_body = serde_json::json!({
        "author": "cr:http-poster",
        "body": "HTTP 経由で返信しました"
    });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/threads/{}/replies", thread_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&post_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        v.get("id").is_some(),
        "POST reply should return id; got {v}"
    );

    // 再度 list は 1 件
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/v1/threads/{}/replies", thread_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 1);
    assert_eq!(v[0]["body"].as_str().unwrap(), "HTTP 経由で返信しました");
}
