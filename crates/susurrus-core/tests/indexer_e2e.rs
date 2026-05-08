//! md 4 種を temp dir に書いて reindex → SQLite クエリで検証する e2e。

use indoc::indoc;
use std::fs;
use std::path::Path;
use susurrus_core::{db::Db, indexer, store::MdStore};

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn reindex_full_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let forum_root = tmp.path().join("forums");
    let db_path = tmp.path().join("susurrus.db");

    // forum
    write(
        &forum_root.join("work/ludiars/_forum.md"),
        indoc! {r#"
            ---
            kind: forum
            id: 0192c5a0-0000-7000-8000-000000000010
            path: work/ludiars
            name: LUDIARS Workspace
            visibility: cernere-group
            group: cg_4f2a
            created_at: 2026-05-08T12:00:00+09:00
            created_by: cr:user
            ---
        "#},
    );

    // channel
    write(
        &forum_root.join("work/ludiars/general/_channel.md"),
        indoc! {r#"
            ---
            kind: channel
            id: 0192c5a0-0000-7000-8000-000000000020
            forum: 0192c5a0-0000-7000-8000-000000000010
            path: work/ludiars/general
            name: general
            topic: 雑談用
            sort: 100
            created_at: 2026-05-08T12:01:00+09:00
            created_by: cr:user
            ---
        "#},
    );

    // thread root
    write(
        &forum_root.join("work/ludiars/general/t_2026-05-08_a3f1.md"),
        indoc! {r#"
            ---
            kind: thread
            id: 0192c5a0-0000-7000-8000-000000000030
            channel: 0192c5a0-0000-7000-8000-000000000020
            forum:   0192c5a0-0000-7000-8000-000000000010
            title: Susurrus 設計レビュー
            tags:
              - design
              - chat
            author: cr:user
            ts: 2026-05-08T12:05:00+09:00
            ---
            # 議題

            設計レビューします。
        "#},
    );

    // reply
    write(
        &forum_root.join("work/ludiars/general/t_2026-05-08_a3f1/m_b9e3f0.md"),
        indoc! {r#"
            ---
            kind: reply
            id: 0192c5a0-0000-7000-8000-000000000040
            thread:  0192c5a0-0000-7000-8000-000000000030
            parent:  0192c5a0-0000-7000-8000-000000000030
            forum:   0192c5a0-0000-7000-8000-000000000010
            channel: 0192c5a0-0000-7000-8000-000000000020
            author: cr:another
            ts: 2026-05-08T12:06:00+09:00
            mentions:
              - cr:user
            ---
            これは返信のテストです **bold** [link](http://x).
        "#},
    );

    let mut db = Db::open(&db_path).unwrap();
    let store = MdStore::new(&forum_root);
    let stats = indexer::reindex_all(&mut db, &store).unwrap();

    assert_eq!(stats.scanned, 4);
    assert_eq!(stats.upserted, 4);
    assert_eq!(stats.unchanged, 0);
    assert_eq!(stats.failed, 0);

    // counts
    let n_forum: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM forum", [], |r| r.get(0))
        .unwrap();
    let n_chan: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM channel", [], |r| r.get(0))
        .unwrap();
    let n_thr: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM thread", [], |r| r.get(0))
        .unwrap();
    let n_rep: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM reply", [], |r| r.get(0))
        .unwrap();
    assert_eq!((n_forum, n_chan, n_thr, n_rep), (1, 1, 1, 1));

    // tags
    let n_tag: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM thread_tag", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n_tag, 2);

    // mention
    let n_men: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM reply_mention", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n_men, 1);

    // last_reply_ts updated on thread
    let last: Option<String> = db
        .conn
        .query_row("SELECT last_reply_ts FROM thread", [], |r| r.get(0))
        .unwrap();
    assert!(
        last.is_some(),
        "last_reply_ts should be set after reply indexing"
    );

    // FTS (reply): trigram tokenizer なので 3 文字以上で検索
    let hit: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM reply_fts WHERE reply_fts MATCH ?1",
            ["返信のテ"],
            |r| r.get(0),
        )
        .unwrap();
    assert!(hit >= 1, "FTS should match 返信のテ in reply body");

    // FTS (thread): タイトル / 本文 どちらでも MATCH できる
    let hit_thr: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM thread_fts WHERE thread_fts MATCH ?1",
            ["設計レビュー"],
            |r| r.get(0),
        )
        .unwrap();
    assert!(hit_thr >= 1, "FTS should match thread title");

    // 2 度目の reindex は unchanged になる (md_hash hit)
    let stats2 = indexer::reindex_all(&mut db, &store).unwrap();
    assert_eq!(stats2.unchanged, 4);
    assert_eq!(stats2.upserted, 0);
}

#[test]
fn prune_after_delete() {
    let tmp = tempfile::tempdir().unwrap();
    let forum_root = tmp.path().join("forums");
    let db_path = tmp.path().join("susurrus.db");

    let forum_md = forum_root.join("space/_forum.md");
    write(
        &forum_md,
        indoc! {r#"
            ---
            kind: forum
            id: 0192c5a0-0000-7000-8000-000000000050
            path: space
            name: tmp
            visibility: public
            created_at: 2026-05-08T12:00:00+09:00
            created_by: cr:user
            ---
        "#},
    );

    let mut db = Db::open(&db_path).unwrap();
    let store = MdStore::new(&forum_root);
    indexer::reindex_all(&mut db, &store).unwrap();
    assert_eq!(
        1,
        db.conn
            .query_row::<i64, _, _>("SELECT COUNT(*) FROM forum", [], |r| r.get(0))
            .unwrap()
    );

    fs::remove_file(&forum_md).unwrap();
    let pruned = indexer::prune_missing(&mut db, &store).unwrap();
    assert_eq!(pruned, 1);
    assert_eq!(
        0,
        db.conn
            .query_row::<i64, _, _>("SELECT COUNT(*) FROM forum", [], |r| r.get(0))
            .unwrap()
    );
}
