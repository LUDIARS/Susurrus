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

v0.0 scaffold 中 (2026-05-08-)。
