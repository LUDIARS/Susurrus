# Susurrus 通信プロトコル

> active=WebRTC+QUIC, sleep=Synergos の経路詳細。

## 1. 経路サマリ

| 状態 | 用途 | 経路 |
|------|------|------|
| ACTIVE | 通常メッセージ / typing / read receipt | WebRTC datachannel (= QUIC) 直結 |
| ACTIVE 確立前 | SDP / ICE 交換 | Synergos relay (WS) 経由 signaling |
| SLEEP | オフライン / 大容量 / catchup | Synergos chain + auto-pull (md ファイル単位) |

ACTIVE で送ったメッセージも **必ず md として commit** し Synergos chain に流す。 これで:
- 他のデバイスにも届く
- 後から検索 / 引用できる
- バックアップに乗る

## 2. ACTIVE 経路

### 2.1 Datachannel ペイロード

CBOR で encode、 magic 4 byte prefix。

| magic | 意味 |
|-------|------|
| `SUMS` | message (md commit notification、 full md は chain 経由) |
| `SUTY` | typing |
| `SURD` | read cursor update |
| `SURX` | reaction add/remove |
| `SUPN` | presence ping |

`SUMS` ペイロード:

```rust
struct SusMs {
    msg_id: Uuid,        // = m_*
    thread_id: Uuid,
    forum_id: Uuid,
    md_hash: [u8; 32],   // blake3
    md_cid: Cid,         // Synergos CID で chain から取れる
    preview: String,     // 最大 256 byte (UI 即時表示用)
    ts: i64,             // unix ms
}
```

受信側は `md_cid` で Synergos に want を出して md 本体を取得 (auto-pull で大抵もう来てる)。

### 2.2 typing

```rust
struct SusTy {
    thread_id: Uuid,
    user_uri: String,    // cr:<uuid>
    until: i64,          // unix ms
}
```

送信側は 2 秒間隔で `until = now + 3000` を送り続ける。 受信側は `until <= now` で表示を消す。

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

## 4. signaling (relay)

WebRTC SDP / ICE は Synergos relay の room ベース broadcast を流用:

- room id: `susurrus:dm:<sorted-user-pair-hash>` または `susurrus:thread:<thread_id>`
- 自分の peer_id + SDP offer/answer + ICE candidate を JSON で投げる
- Trickle ICE
- DTLS / SRTP 鍵は Synergos の真性認証 (ed25519 PeerId) で「offer の sender が本人か」 を別 chain entry で署名検証

## 5. 認証

- Cernere token は core daemon 起動時に 1 度提示し、 user_uri を確定。
- peer 間メッセージは Synergos 真性認証 (PeerId = blake3(ed25519 pub)) で peer 単位で検証済み。
- 「peer X が user Y であること」 の紐付けは Cernere 側で管理。 Susurrus は Cernere に問い合わせて map する。

## 6. 失敗 / リトライ

| 事象 | 対応 |
|------|------|
| ACTIVE で `SUMS` 通知失敗 | SLEEP 経路 (Synergos chain) に投げる、 後で peer がオンラインになれば届く |
| Synergos chain commit 失敗 | core 内 retry queue に積む (永続) |
| md 読み出し失敗 (chain にないがメタはある) | 受信側で「読み込み中」 表示、 BSW1 chunk pull で待つ |

## 7. 将来 (Spatial Chat) の枠

- WebRTC datachannel + 同 PeerConnection に audio track を相乗り
- 位置情報は overlay SDK のホストが定期送信 (CBOR magic `SUSP`)
- 距離減衰は受信側 SDK でかける、 サーバ側計算なし
