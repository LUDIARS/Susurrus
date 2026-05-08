//! compose API で forum→channel→thread→reply を作って query で読み返す e2e。

use susurrus_core::{compose, db::Db, query, store::MdStore};
use susurrus_md::Visibility;

#[test]
fn create_full_chain_then_query() {
    let tmp = tempfile::tempdir().unwrap();
    let forum_root = tmp.path().join("forums");
    let db_path = tmp.path().join("susurrus.db");

    std::fs::create_dir_all(&forum_root).unwrap();
    let mut db = Db::open(&db_path).unwrap();
    let store = MdStore::new(&forum_root);

    let forum = compose::create_forum(
        &mut db,
        &store,
        "work/ludiars",
        "LUDIARS Workspace",
        Visibility::CernereGroup,
        Some("cg_4f2a".into()),
        "cr:user-uuid",
    )
    .unwrap();

    let channel = compose::create_channel(
        &mut db,
        &store,
        forum.id,
        &forum.path,
        "general",
        "雑談用",
        100,
        "cr:user-uuid",
    )
    .unwrap();

    let thread = compose::create_thread(
        &mut db,
        &store,
        forum.id,
        channel.id,
        &channel.path,
        "Susurrus 設計レビュー",
        "# 議題\n\n設計レビューします。",
        vec!["design".into(), "chat".into()],
        "cr:user-uuid",
    )
    .unwrap();

    // thread の md_path を SQLite から拾う (compose は path を返さないので)
    let thread_md_path: String = db
        .conn
        .query_row(
            "SELECT md_path FROM thread WHERE id = ?1",
            [thread.id.to_string()],
            |r| r.get(0),
        )
        .unwrap();

    let _reply = compose::create_reply(
        &mut db,
        &store,
        forum.id,
        channel.id,
        thread.id,
        &thread_md_path,
        thread.id, // root への返信
        "返信します **bold** [link](http://x).",
        "cr:another",
        vec!["cr:user-uuid".into()],
        Vec::new(),
    )
    .unwrap();

    // query で読み返す
    let forums = query::list_forums(&db.conn).unwrap();
    assert_eq!(forums.len(), 1);
    assert_eq!(forums[0].name, "LUDIARS Workspace");

    let channels = query::list_channels(&db.conn, &forum.id.to_string()).unwrap();
    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].name, "general");

    let threads = query::list_threads(&db.conn, &channel.id.to_string(), 50, 0).unwrap();
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].title, "Susurrus 設計レビュー");
    assert_eq!(threads[0].reply_count, 1);
    assert!(threads[0].last_reply_ts.is_some());
    assert_eq!(threads[0].tags, vec!["chat".to_string(), "design".to_string()]);

    let replies = query::list_replies(&db.conn, &thread.id.to_string()).unwrap();
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].author, "cr:another");
    assert_eq!(replies[0].mentions, vec!["cr:user-uuid"]);

    let hits = query::search_replies(&db.conn, "返信し", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert!(hits[0].snippet.contains("返信"));

    // body 取得 (thread root + reply)
    let tb = query::read_thread_body(&db.conn, &store, &thread.id.to_string()).unwrap();
    assert!(tb.body.contains("議題"));
    assert!(tb.md_path.ends_with(".md"));
    let rb = query::read_reply_body(&db.conn, &store, &replies[0].id).unwrap();
    assert!(rb.body.contains("返信します"));
}
