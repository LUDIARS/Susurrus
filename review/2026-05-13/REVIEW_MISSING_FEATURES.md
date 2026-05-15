# REVIEW_MISSING_FEATURES (Susurrus, 2026-05-13)

**評価: B**

README v1.0 ✅ と実装の照合で未着手 / scaffold 止まりが多い:

1. **Cernere verify** (`cernere.rs:11`) — `unimplemented!()`。
2. **FS watcher** — `PROTOCOL.md:94` で謳う notify 経路が無く、 `Cargo.toml` 依存も無し。 `reindex_path` の trigger 不在。
3. **incoming file → reindex pipeline** (`bridge.rs:69`) — 消費 task が起動経路 (`lib.rs:13`) に無い。
4. **forum_subscription 管理 API** — `db.rs:202` 表のみ、 CRUD 無し。
5. **read cursor 操作 API** (`db.rs:185`, `payload.rs:32`) — 表 + 型のみ。
6. **reaction (SUX1)** — payload + 表のみ、 UI 経路無し。
7. **window-attach** (`SPEC.md:117-120`) — HWND 追従未実装。
8. **DM forum 自動生成** (`MD-SCHEMA.md:127`) — compose ヘルパー無し。
9. **`spec/SPATIAL.md` 空**。

Roadmap 乖離: `v1.0 Synergos QUIC ✅` は `SynergosBus` ローカル止まり、 `lib.rs:4-7` 自身が「Synergos に PR 必要」と書く。 `v1.0 Audio ✅` は CI Linux のみで macOS/Windows 未検証。
