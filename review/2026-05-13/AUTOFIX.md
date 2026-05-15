# AUTOFIX (Susurrus, 2026-05-13)

> ソースコード修正禁止のため、本ファイルは AUTOFIX 候補の列挙のみ。 `autofix_count = 0`。

## 安全範囲で AUTOFIX 可能な候補 (列挙のみ・実行しない)

1. `crates/susurrus-synergos/src/lib.rs:3-7` — ヘッダ doc コメントが「v0.3 = SLEEP only / ACTIVE は Synergos PR 待ち」と古い。v1.0 で `SynergosBus` 実装済みの旨に更新するだけ。
2. `README.md:55` — Roadmap 表の v0.3 行「ACTIVE path は Synergos PR 待ち」 を v1.0 行と整合させる文言修正。
3. `README.md:78` — `CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo run ...` を PowerShell 用 `$env:CMAKE_POLICY_VERSION_MINIMUM='3.5'; cargo run ...` も併記。
4. `README.md:82-88` — env 一覧に `SUSURRUS_MEMORIA_TOKEN` を追記 (`state.rs:61` で読まれているが未記載)。
5. `crates/susurrus-md/src/lib.rs:160-167` — `split_frontmatter` の CRLF コメント「ここでは error にせず assume LF」を実際に `replace("\r\n","\n")` を行う実装にするか、最低限「呼び出し側で正規化必須」と doc を明確化。
6. `spec/SPATIAL.md` — 空ファイル。「v0.0 では未確定」 と 1 行入れて意図を明示。
7. `spec/SPEC.md:78-104` および `README.md:5,11-15` の「ACTIVE = WebRTC + QUIC」記述を「ACTIVE = Synergos QUIC stream (WebRTC は Spatial Chat v1.0+ で検討)」へ書き換え、`PROTOCOL.md:1-12` と整合させる。
8. `crates/susurrus-synergos/src/backend.rs:257` — broadcast 内 `project_id: "susurrus"` を `self.backend` 経由で持つ config 値に差し替える (構造変更が必要なので AUTOFIX 範囲外、ここでは指摘のみ)。
9. `.github/workflows/ci.yml:165-176` — `cargo audit` の `continue-on-error: true` を外す代わりに `--ignore RUSTSEC-XXXX` allow-list を導入 (要 advisory ID 確定)。
10. `crates/susurrus-core/src/cernere.rs` — `unimplemented!()` のままだと panic 経路。少なくとも `anyhow::bail!("cernere verify not yet implemented")` に置き換えて起動継続できるようにする (本来の AUTOFIX 範囲外、要レビュー)。

## 適用方針

本リポは「ソースコード修正禁止」の指示下のため、autofix 列挙のみ。`/ludiars-review` の本リポ自動 PR 起動時は上記 1-7 (doc 系) のみを安全範囲とし、8-10 はレビュー後の手動 PR 推奨。

`autofix_count: 0`
