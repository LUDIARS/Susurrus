# Spatial Chat Mode (v1.0+)

> v1.0 でフル実装。 v0.x は protocol skeleton + SDK 位置 API のみ提供。

## 1. ユースケース

- ゲーム内 / VR 空間内でアバター位置に応じて聞こえ方を変える音声チャット
- ツール内オーバーレイ (Pictor / Ergo / Unity) で「ある場所にいる人だけに話しかける」 ボイス
- 距離減衰 + パン (ステレオ) は **受信側 SDK** で計算 (server なしで完結)

## 2. データモデル

### 2.1 SpatialPosition

```rust
pub struct SpatialPosition {
    pub x: f32, pub y: f32, pub z: f32,
    pub qx: f32, pub qy: f32, pub qz: f32, pub qw: f32, // optional orientation
}
```

任意座標系 (= forum メタに `coord_system` を追加して定義)。 v1.0 初期は単位 = メートル想定。

### 2.2 SusSpatial wire payload

```rust
pub struct SusSpatial {
    pub forum_id: Uuid,
    pub user_uri: String,
    pub x: f32, pub y: f32, pub z: f32,
    pub qx: f32, pub qy: f32, pub qz: f32, pub qw: f32,
    pub ts_ms: i64,
}
```

magic = `SUS1` (susurrus-rt::Magic::Spatial)。 stream は 200ms 間隔で送る long-lived stream。

## 3. 距離モデル

SDK 側で 3 つから選択 (各 forum 設定):
- `linear(min, max)` — min 以下 = 1.0、 max 以上 = 0.0、 線形
- `inverse(ref, max)` — `ref / max(d, ref)`、 上限 1.0
- `inverse_square(ref, max)` — `(ref / max(d, ref))^2`

[`susurrus-sdk::spatial::linear_attenuation`] が初期実装。

## 4. 部屋 / セクタ

forum メタに optional `spatial: { kind: "rooms" | "free", rooms: [...] }`:
- `rooms` = 列挙された部屋 (聞こえる範囲は同部屋のみ + 部屋間ドアの開閉)
- `free` = 距離だけで決まる無制限空間 (オープンワールド系)

## 5. 音声経路

選択肢 (v1.0 で評価):
- (A) WebRTC datachannel + audio track (SCTP/SRTP)
- (B) Synergos QUIC datagram + Opus フレームを自前パケタイズ
- (C) 既存 Mumble/Jitsi 等を別 daemon として組み込み Susurrus が位置だけ流す

第一候補は (B) — Synergos の P2P 経路を全て使い、 NAT 越えとセキュリティを既存と一貫させる。 (A) は browser peer 互換が必要になったら追加。

## 6. プライバシ

- 「現在 Spatial に参加中」 を opt-in で choose-and-join。
- 位置 + 音声は **forum 内 ACTIVE peer 限定** (chain には載せない、 SLEEP 経路にも乗せない)。
- 録音は SDK 側 / OS 側で個別管理 (Susurrus 側は無し)。

## 7. v1.0 のマイルストーン

| 項目 | 状態 |
|------|------|
| `SusSpatial` payload + `SUS1` magic 登録 | ✅ susurrus-rt v0.2 で先取り |
| SDK `report_position` API + C ABI | ✅ susurrus-sdk v0.5 で先取り |
| 距離減衰関数 (linear) | ✅ |
| 距離減衰関数 (inverse / inverse_square) | TODO |
| 部屋 / セクタ概念 + forum メタ拡張 | TODO |
| 音声 Opus codec + (B) 経路実装 | TODO |
| Synergos QUIC datagram 拡張 (Synergos PR) | TODO |
| Pictor / Ergo / Unity サンプルアプリ | TODO |
