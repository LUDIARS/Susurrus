# REVIEW (Susurrus, 2026-05-13)

| カテゴリ | 評価 | 主旨 |
|---|---|---|
| Design | B | 4 階層 + md 正本 + MessageBus/SynergosBackend trait の構造は健全。ただし ACTIVE 経路の README/SPEC ↔ PROTOCOL 間で WebRTC vs Synergos QUIC の食い違い、forum_subscription / Cernere peer 紐付け / SPATIAL.md 等の設計穴あり |
| Vulnerability | C | loopback HTTP が `CorsLayer::permissive()` + 認証なし + `author` 自己申告 = ローカル CSRF + なりすまし容易。Cernere verify が unimplemented。Synergos broadcast の project_id ハードコード |
| Implementation | B | async trait 内 `futures_lite::block_on` の deadlock 懸念、incoming md → reindex pipeline 未配線、publish の md_path 二度引きなど。SQLite/FTS5/CBOR 周りの基礎は丁寧 |
| Missing Features | B | Cernere verify / FS watcher / 受信 → index 配線 / subscription API / read cursor / reaction CRUD / window-attach / DM 自動生成 / SPATIAL.md が未着手。README の v1.0 ✅ は実装と乖離 |
| Quality | B | 3-OS CI + clippy `-D warnings` + 30 件テスト + smoke job は標準的。Workspace の sibling repo path 依存、silent Synergos fallback、unwrap/expect 散在が要整理 |

## weighted_score (重み: Design 0.20 / Vuln 0.30 / Impl 0.20 / Missing 0.15 / Quality 0.15)

`A=4, B=3, C=2, D=1` 換算:
- 0.20*3 (B) + 0.30*2 (C) + 0.20*3 (B) + 0.15*3 (B) + 0.15*3 (B)
- = 0.60 + 0.60 + 0.60 + 0.45 + 0.45 = **2.70 / 4.00 (= 67.5)**

## 重要所見 (top 5)

1. **loopback HTTP の CSRF/なりすまし** (`susurrus-tauri/src-tauri/src/http.rs:38, 86-138`) — `CorsLayer::permissive()` と `author` クライアント信頼の合わせ技でローカル悪意ページから任意ユーザとして投稿可能。
2. **`cernere::verify` が `unimplemented!()`** (`crates/susurrus-core/src/cernere.rs:11`) — README の中核機能が panic ハザード。
3. **incoming md を reindex する task が無い** (`crates/susurrus-synergos/src/bridge.rs:69`, `susurrus-tauri/src-tauri/src/lib.rs:13`) — 他 peer から届いた md が UI に反映されない (= 受信側がほぼ動かない)。
4. **ACTIVE 経路の仕様食い違い** — SPEC は WebRTC、PROTOCOL は Synergos QUIC、実装は Synergos QUIC。README/SPEC を PROTOCOL に揃える必要。
5. **Workspace path dep** (`Cargo.toml:50-52`) — `synergos-core/synergos-net` が path のまま。`feedback_cross_repo_path_dep.md` の運用ルールに反する。

## 件数

- High: 5 (上記)
- Medium: 7 (silent Synergos fallback, FS watcher 未実装, forum_subscription API 無し, read_cursor API 無し, reaction CRUD 無し, broadcast project_id ハードコード, save_to_memoria のクライアント信頼 title)
- Low: 8 (DM hash 未指定, SUSURRUS_USER 信頼, cargo audit non-blocking, async trait block_on, store::norm_path doc, compose の FS+DB 非 atomic, README の cmake env 表記, lib.rs ヘッダコメントの古さ)

総計 20 件。
