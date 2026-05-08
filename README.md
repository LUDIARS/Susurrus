# Susurrus (Su)

> "Susurrus" — ラテン語で「ささやき」。 LUDIARS 内のローカル先行チャットサービス。

Cernere 認証 + Synergos Core 依存。 active 時は WebRTC + QUIC datachannel でリアルタイム会話、休眠中は Synergos の P2P async (Exchange + chain + auto-pull) でメッセージを配送する。

データの正本は **Markdown ファイル**。フォーラム帰属は frontmatter 先頭で表現する (ドキュメント運用 + git/Synergos 同期に乗せやすい)。 ツリー / インデックス / FTS / typing presence 等は SQLite キャッシュに置く。

## 機能ハイライト

- **Cernere 認証** — token verify でユーザ識別 (個人データ非保管)
- **ハイブリッド配送** — active=WebRTC+QUIC realtime / 休眠=Synergos async
- **Discord 風フォーラム + Slack 風スレッド** — forum/channel/thread/reply の 4 階層
- **入力中インジケータ** — QUIC 接続中のみ
- **Memoria 連携** (opt-out 可) — チャット → ブクマ / 日記 / Dig 連携
- **Window detach + window-attach** — 任意ウィンドウに取り付けて会話できる
- **Overlay SDK** — ゲームエンジン / ツール / ビューアー (Pictor / Ergo / Unity 等) から呼べる
- **(将来) Spatial Chat Mode** — 空間音響 + 距離減衰、まだ作らない

## 依存

- [Synergos](../Synergos) — `synergos-core` / `synergos-net` を path 依存
- [Cernere](../Cernere) — token verify
- [Memoria](../Memoria) — opt-in 連携

## 仕様書

- [spec/SPEC.md](spec/SPEC.md) — 機能仕様
- [spec/MD-SCHEMA.md](spec/MD-SCHEMA.md) — Markdown frontmatter / 配置仕様
- [spec/DB-SCHEMA.md](spec/DB-SCHEMA.md) — SQLite キャッシュスキーマ
- [spec/PROTOCOL.md](spec/PROTOCOL.md) — QUIC / WebRTC / Synergos 経路の詳細

## Workspace

| crate | 役割 |
|---|---|
| `susurrus-core` | daemon。 Cernere auth + Synergos bridge + SQLite + md store + presence |
| `susurrus-md` | md frontmatter parser/writer + tree resolver |
| `susurrus-rt` | active 時の WebRTC + QUIC datachannel + typing |
| `susurrus-tauri` | Tauri 2 GUI。 multi-window で detach + window-attach |
| `susurrus-sdk` | overlay SDK (Rust crate + 後で C ABI) |
| `susurrus-memoria` | Memoria 双方向同期 (opt-out 可) |

## Status

2026-05-08 時点 (v1.0 へ向けた scaffold 完了):

| Milestone | 状況 |
|---|---|
| v0.0 spec + workspace | ✅ |
| v0.1 daemon + md + SQLite + minimal Tauri GUI | ✅ |
| v0.2 realtime scaffold (rt::magic/payload/link/typing/transport/MockBus) | ✅ |
| v0.3 Memoria 連携 (HTTP client + opt-out) | ✅ |
| v0.3 Synergos 連携 (SLEEP path = chain commit) | ✅ ACTIVE path は Synergos PR 待ち |
| v0.4 Window detach (Tauri multi-window) | ✅ |
| v0.5 Overlay SDK (Rust crate) | ✅ |
| v0.6 Overlay SDK C ABI (cdylib) | ✅ |
| v1.0 Spatial Chat (protocol skeleton + SDK 位置 API) | ✅ 音声 codec/audio backend は TODO |
| Tauri loopback HTTP server (axum) | ✅ |

`cargo test --workspace` → 30 / 30 pass、 `cargo build -p susurrus-tauri` ok、 frontend ビルド ok。

## 起動

```bash
cd susurrus-tauri/frontend
npm install && npm run build      # 初回のみ
cd ../..
cargo run -p susurrus-tauri        # Tauri shell + axum HTTP server (17370) 起動
```

オプション env:
- `SUSURRUS_DATA` データディレクトリ (default: OS data_local_dir/Susurrus)
- `SUSURRUS_USER` Cernere user URI (default: cr:local-user)
- `SUSURRUS_SYNERGOS=1` Synergos daemon (synergos-core) に IPC 接続 (default は Noop)
- `SUSURRUS_MEMORIA_ENDPOINT` Memoria endpoint URL (default: http://127.0.0.1:5180)
- `SUSURRUS_MEMORIA_DISABLED` 値があれば Memoria 連携を完全 off
- `SUSURRUS_LOCAL_PORT` SDK 用 HTTP server port (default 17370)

## SDK (overlay)

```rust
use susurrus_sdk::Susurrus;
let s = Susurrus::local_default();
let id = s.send_reply("<thread_id>", "cr:user", "hello").await?;
```

C ABI からは `susurrus_create / send_reply / report_position / destroy`。 ヘッダは `susurrus_sdk.h` を生成予定 (cbindgen)。

