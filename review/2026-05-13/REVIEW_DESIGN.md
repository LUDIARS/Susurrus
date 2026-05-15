# REVIEW_DESIGN (Susurrus, 2026-05-13)

**評価: B**

良: 4 階層 + frontmatter 仕様は一貫し `compose.rs:34` で素直に実装。 `MessageBus` (`transport.rs:25`) と `SynergosBackend` (`backend.rs:15`) trait で Noop/Mock/Ipc 切替可。 md 正本 / DB キャッシュ原則を `indexer::reindex_all` + `prune_missing` で運用化。 5 magic を `payload.rs` で型化 + CBOR round-trip test 完備。

懸念:
1. **ACTIVE 経路の食い違い**: `README.md:5` / `SPEC.md:78-95` は WebRTC+QUIC、 `PROTOCOL.md:1-12` は Synergos QUIC へ pivot。 実装 (`SynergosBus`) は後者準拠で SPEC/README が古い。
2. **forum_subscription 設計空白**: `db.rs:202` 表のみ、 catchup 駆動と編集 API 不在。
3. **Cernere peer 紐付け**: `cernere.rs:11` が `unimplemented!()`、 `presence` 表 (`db.rs:167`) も 1:1 で複数 PC 同一 user (SPEC §2) 表現不可。
4. **window-attach 未着手**: `SPEC.md:117-120` の HWND 追従が独立 webview 止まり (`commands.rs:389`)。
5. **`spec/SPATIAL.md` 空**: 実装先行で設計が追従していない。
