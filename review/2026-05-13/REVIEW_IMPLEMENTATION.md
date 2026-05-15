# REVIEW_IMPLEMENTATION (Susurrus, 2026-05-13)

**評価: B**

懸念:
1. **async trait 内 `futures_lite::block_on`** (`backend.rs:75, 172, 216`): Mutex 取得のため block_on、 ランタイムスレッドから deadlock 可。 `async fn` か `OnceCell` に。
2. **incoming md → reindex 未配線** (`bridge.rs:69-72`, `lib.rs:13`): `subscribe_incoming` 消費 task が無く auto-pull 受信が SQLite に反映されない。
3. **publish md_path 二度引き** (`http.rs:140-160`): `compose::create_reply` が abs を返せば race + ラウンドトリップ解消。
4. **FS write と DB upsert 非 atomic** (`compose.rs:71-197`): write 成功 / DB 失敗で md だけ残る挙動未明文化。
5. **`save_to_memoria` の title 信頼** (`commands.rs:340-369`): SQL で実 title 引かないと改ざん可。

良: `Db::open` で WAL/foreign_keys ON/mmap=256MB 一括、 FTS5 trigram、 `payload.rs` の `serde_bytes` 適用。
