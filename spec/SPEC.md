# Susurrus 機能仕様書 (v0.0)

> 2026-05-08 起草。 ユーザ要件 + Synergos / Cernere / Memoria 既存設計から導出。 仕様変更は本ファイルへの PR を正本とする。

## 1. 全体像

Susurrus は **ローカル daemon + Tauri GUI** からなるチャットサービス。 ユーザは LUDIARS 内のメンバーと P2P でメッセージを送受信する。

```
┌────────────────────────────────────────────────────────────────────┐
│  Susurrus host (各 PC)                                             │
│                                                                    │
│  ┌──────────────┐    IPC     ┌──────────────────────────────────┐  │
│  │ susurrus-tauri│◀──────────▶│ susurrus-core (daemon)           │  │
│  │  (multi-win) │            │  ├─ Cernere auth                  │  │
│  └──────┬───────┘            │  ├─ md store (forums/**/*.md)     │  │
│         │                    │  ├─ SQLite cache (index/FTS)      │  │
│         │ overlay SDK        │  ├─ susurrus-rt (WebRTC+QUIC)     │  │
│         ▼                    │  ├─ susurrus-md (parser/writer)   │  │
│   game / tool / viewer       │  └─ Synergos bridge (Exchange)    │  │
│                              └────────┬─────────────────────────┘  │
└─────────────────────────────────────┬─┴───────────────────────────┘
                                      │
                  active ▼            ▼ sleep
              ┌───────────────┐   ┌────────────────┐
              │ WebRTC / QUIC │   │ Synergos chain │
              │ datachannel   │   │ + auto-pull    │
              └───────────────┘   └────────────────┘
```

## 2. 認証 / アイデンティティ

- **Cernere token** で初期ログイン。 user URI = `cr:<uuid>`。
- 個人データ (display name / avatar 等) は Cernere からのみ取得。 Susurrus 自身は保持しない (LUDIARS § 個人データ規約)。
- Synergos の `PeerId` (blake3 of ed25519 pub) は Cernere user に **複数紐付け可能** (PC ごと)。 紐付けは Cernere 側に登録。

## 3. データモデル

### 3.1 階層

```
forum            (= Discord guild + forum 統合。 例: forums/work/ludiars)
└─ channel       (= Discord channel。 例: forums/work/ludiars/general)
   └─ thread     (= Slack thread / X リプライチェーン root)
      └─ reply   (thread 内の枝分かれ可。 parent で指す)
```

- forum / channel / thread / reply はすべて **Markdown ファイル**。
- thread root と reply は同じ schema (frontmatter で `parent` の有無により判定)。
- DM は専用 forum (`forums/_dm/<user-pair-hash>`) として扱う。

### 3.2 ファイル配置

```
$SUSURRUS_DATA/
├─ forums/
│  └─ work/
│     └─ ludiars/
│        ├─ _forum.md             ← forum メタ (アクセス制御 / 説明)
│        ├─ general/
│        │  ├─ _channel.md        ← channel メタ
│        │  ├─ t_2026-05-08_a3f1.md       ← thread root
│        │  ├─ t_2026-05-08_a3f1/
│        │  │  ├─ m_b9e3f0.md     ← reply
│        │  │  └─ m_c1d2e3.md
│        │  └─ ...
│        └─ random/
└─ db/susurrus.db                 ← SQLite キャッシュ (再生成可)
```

詳細は [MD-SCHEMA.md](MD-SCHEMA.md)。

## 4. 配送経路

### 4.1 状態機械

```
        peer online & 互いに recent
   ┌────────────────────────────┐
   │             ACTIVE         │   ← WebRTC+QUIC datachannel 確立
   │ realtime msg / typing / ack│
   └────────────┬───────────────┘
                │ idle > N 秒 / peer offline / NAT 切断
                ▼
   ┌────────────────────────────┐
   │             SLEEP          │
   │ Synergos chain で md offer │
   │ auto-pull で受信側へ       │
   └────────────┬───────────────┘
                │ either side online & 直結成立
                ▲
                └── 戻る
```

- ACTIVE は最低限の signaling だけ Synergos relay 経由 (SDP 交換)。 以降は WebRTC datachannel = QUIC でデータ平面。
- 同一メッセージは ACTIVE で送れた場合も「正本 md」として SLEEP 経路にも commit する (= Synergos chain にも乗る)。これで他の peer / オフラインだったメンバーへの届く保証が出る。

### 4.2 typing indicator

- ACTIVE の datachannel 上で `Typing { thread, until }` を 2 秒間隔で送る (until = now + 3s)。
- SLEEP では一切流さない。
- 受信側は `until` を超えたら表示を消す。

詳細は [PROTOCOL.md](PROTOCOL.md)。

## 5. UI 仕様

### 5.1 メイン window

- 左: forum tree (展開 / 折りたたみ)
- 中: channel 内の thread 一覧 (Discord フォーラム風カード)
- 右: 選択中 thread の reply chain (Slack スレッド風縦並び)

### 5.2 detach

- 任意の thread / channel を別 window に切り出し。
- detach 時のオプション:
  - **standalone** — 独立 window として配置
  - **attach to window** — OS の HWND / NSWindow / X11 window に親付け (overlay) し、対象 window に追従して移動
- attach の対象選択は Tauri から OS API を呼ぶ (Windows: `EnumWindows` + title/process matching、 macOS / Linux 後回し)。

### 5.3 overlay SDK 経由

- ホストアプリ (Pictor / Ergo / Unity 等) が `susurrus-sdk` を組み込むと、 自前 GUI に Susurrus widget を埋め込める (renderer 自前 / IPC は同じ daemon)。
- SDK はまず Rust crate、 後で C ABI を切る。
- ゲーム内チャット = overlay SDK + spatial chat (将来) の組み合わせ。

## 6. Memoria 連携

- 既定 **opt-in** (= 既定 on、 ユーザが opt-out できる)。
- 連携機能 (v0.1 で実装したい範囲):
  - `/save-to-memoria` でメッセージ → Memoria ブクマ
  - `/dig <url>` で Memoria Dig 結果を thread 内へ返す
  - メンションで Memoria task delegation (将来)
- 実装は `susurrus-memoria` crate (Memoria HTTP API を叩く client)。
- Memoria 側の認証は Cernere token を流用。

## 7. Spatial Chat Mode (将来)

- 仕様は v0.0 では確定しない。設計だけ枠を確保:
  - `susurrus-rt` の WebRTC datachannel に音声 SRTP を相乗り
  - 距離減衰 / pan は受信側 SDK でかける
  - 「位置」は overlay SDK のホストが供給 (ゲームエンジン側の player position)
- v0.0 では `spec/SPATIAL.md` を空ファイルにしておく。

## 8. 非機能要件

- **オフライン優先** — Synergos chain に既に乗ったログは再起動後即読める。 SQLite キャッシュは破棄→再構築可能。
- **マルチデバイス** — 1 ユーザ複数 peer。 同じ md を全 peer が pull (Synergos auto-pull)。 既読 cursor は per-peer。
- **個人データ** — 名前 / アイコン等は Cernere、 メッセージ author は `cr:<uuid>` のみ保存。
- **セキュリティ** — peer 認証は Synergos の S1 真性認証 (ed25519 + blake3 PeerId) を流用。 Susurrus 側で再認証は不要。
- **バックアップ** — md ファイル群を git or 任意手段で持ち出せる (ドキュメント設計の主目的)。

## 9. ロードマップ

| Milestone | 内容 |
|---|---|
| **v0.0** | scaffold、 spec、 DB schema、 md schema (本書) |
| v0.1 | susurrus-core daemon + susurrus-md + SQLite cache + minimal Tauri GUI (forum tree + thread view) |
| v0.2 | susurrus-rt (Synergos relay 経由 WebRTC SDP + QUIC datachannel) + typing |
| v0.3 | Memoria 連携 (opt-out toggle 込み) |
| v0.4 | window detach + window attach (Windows のみ) |
| v0.5 | overlay SDK (Rust crate)、 KuzuSurvivors / AdventureCube に試験統合 |
| v0.6 | overlay SDK の C ABI、 Unity / Pictor 統合 |
| v1.0 | Spatial Chat Mode 設計開始 |

## 10. 開いている設計判断

- forum 階層の最大深さは無制限か上限を切るか (現案: 無制限、 path で `/` 区切り)
- thread ID generation: ULID か UUIDv7 か → **UUIDv7** で確定 (時系列ソート可能 + Synergos と互換)
- DM の forum path 命名: `forums/_dm/<sorted-user-pair-hash>` で確定
- forum メンバ管理 / 招待: v0.1 では Cernere group を流用、 forum メタにグループ ID を書く
