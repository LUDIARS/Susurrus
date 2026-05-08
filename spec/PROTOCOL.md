# Susurrus 通信プロトコル

> active=Synergos QUIC bidi stream / sleep=Synergos chain。
> WebRTC datachannel は **Spatial Chat (v1.0+) で audio track と同居する用途のみ** に絞り、
> v0.2 のテキストチャットでは採用しない (= 既存の Synergos QUIC をフル活用する)。

## 1. 経路サマリ

| 状態 | 用途 | 経路 |
|------|------|------|
| ACTIVE | 通常メッセージ / typing / read receipt | Synergos QUIC bidi stream (新規 magic) |
| SLEEP | オフライン / 大容量 / catchup | Synergos chain + auto-pull (md ファイル単位) |

ACTIVE で送ったメッセージも **必ず md として commit** し Synergos chain に流す。 これで:
- 他のデバイスにも届く
- 後から検索 / 引用できる
- バックアップに乗る

ACTIVE は notification / typing 等の **低レイテンシ通知**専用。 本体は md → Synergos chain。

## 2. Synergos QUIC stream protocol

### 2.1 Stream magic

Synergos の既存 magic (HLO1/DHT1/TXFR/GSP1/BSW1) と並ぶ susurrus 専用 5 種:

| magic | 意味 | 方向 |
|-------|------|------|
| `SUM1` | message commit notification | bidi (片側 send + ack) |
| `SUT1` | typing | uni (broadcast 風) |
| `SUR1` | read cursor update | uni |
| `SUX1` | reaction add/remove | uni |
| `SUP1` | presence ping | bidi (RTT 計測) |

各 stream は magic 4 byte の後に CBOR 1 frame を載せる。 stream 寿命は基本 1 message = 1 stream (短命) だが、 typing / presence は long-lived stream を再利用しても良い。

### 2.2 ペイロード型

すべて `serde` でエンコード (CBOR ワイヤ、 JSON テスト)。

```rust
struct SusMsg {
    msg_id: Uuid,        // = m_*
    thread_id: Uuid,
    forum_id: Uuid,
    md_hash: [u8; 32],   // blake3
    md_cid: [u8; 32],    // Synergos CID で chain から取れる
    preview: String,     // 最大 256 byte (UI 即時表示用)
    ts_ms: i64,          // unix ms
}

struct SusTyping {
    thread_id: Uuid,
    user_uri: String,    // cr:<uuid>
    until_ms: i64,       // unix ms。 受信側はこれを過ぎたら表示を消す
}

struct SusRead {
    thread_id: Uuid,
    user_uri: String,
    last_read_reply_id: Option<Uuid>,
    last_read_ts_ms: i64,
}

struct SusReact {
    reply_id: Uuid,
    thread_id: Uuid,
    emoji: String,
    user_uri: String,
    add: bool,           // false = 取り消し
    ts_ms: i64,
}

struct SusPing {
    nonce: u64,          // RTT 計測の符合
    ts_ms: i64,
}
```

受信側は `SUM1` を受けたら `md_cid` で Synergos に want を出して md 本体を取得 (auto-pull で大抵もう来てる)。

### 2.3 typing の挙動

送信側は 2 秒間隔で `SusTyping { until_ms = now + 3000 }` を送り続ける。 受信側は `until_ms <= now` で表示を消す。 stream は再利用可、 切れたら再 dial。

## 3. SLEEP 経路

Synergos の既存配送機能をそのまま使う:

1. core が md ファイルを `forums/**/m_*.md` に書く
2. core が Synergos に `PublishUpdate` IPC を投げる
3. Synergos が ledger に Offer を登録 + gossip で `FileOffer/CatalogUpdate` を bcast
4. 受信側 Synergos が auto-pull (FileWant → TXFR)
5. 受信側 Susurrus core は FS watcher で新しい md を検出して SQLite に index

→ Susurrus 側は「md 書く / md 読む」 だけで完結。 Synergos に依存し切る。

### 3.1 catchup

- 起動時、 各 forum で「自分が持っていない md」 を Synergos chain から枚挙して順に pull。
- `forum_subscription` 表に基づいてどの forum を pull するか決める。

## 4. peer ↔ user 紐付け

- Cernere token は core daemon 起動時に 1 度提示し、 user_uri を確定。
- peer 間メッセージは Synergos 真性認証 (PeerId = blake3(ed25519 pub)) で peer 単位で検証済み。
- 「peer X が user Y であること」 の紐付けは Cernere 側で管理。 Susurrus は Cernere に問い合わせて map する。

## 5. LinkState 機械

```
    peer online & 互いに recent
   ┌────────────────────────────┐
   │             ACTIVE         │   ← Synergos QUIC stream で SUM1/SUT1 等を流せる
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

実装上は `susurrus-rt::link::LinkState` enum + per-peer の最終 RTT/seen を SQLite `presence` 表へ反映。

## 6. 失敗 / リトライ

| 事象 | 対応 |
|------|------|
| ACTIVE で `SUM1` 通知失敗 | SLEEP 経路 (Synergos chain) には既に投げているので peer がオンラインになれば届く。 retry 不要 |
| Synergos chain commit 失敗 | core 内 retry queue に積む (永続) |
| md 読み出し失敗 (chain にないがメタはある) | 受信側で「読み込み中」 表示、 BSW1 chunk pull で待つ |

## 7. v1.0+ (Spatial Chat) で WebRTC を導入する理由

- 音声 track (Opus) を SCTP / SRTP で同居させたい
- ブラウザ peer 含めたい場合の互換性 (将来オプション)
- 距離減衰など spatial mixing は受信側 SDK でかける、 サーバ側計算なし

これらは **Synergos QUIC datagram + 自前音声 track でも代替可** なので、 WebRTC 導入は best-effort。 v0.2 のテキストチャット部分とは独立して評価する。
