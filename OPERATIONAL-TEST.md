# Susurrus Operational Smoke Test

> **対象**: 2 物理 PC で Susurrus + Synergos のフル経路 (chain auto-pull / ACTIVE QUIC stream / 音声) が動くかの手動確認手順。
> CI で自動化されている #1 (1 台 + NoopBackend) は別経路で常時走っている → [`.github/workflows/ci.yml`](.github/workflows/ci.yml) の `smoke` job。

## 0. このドキュメントが扱う scenario

| ID | 内容 | 自動化 |
|----|------|------|
| #1 | 1 台 + Synergos = NoopBackend | ✅ CI smoke |
| #2 | 同 PC 2 instance + Synergos LAN 2 ノード | (省略、 必要時に書き足す) |
| **#3** | **2 物理 PC + Synergos via Cloudflare Tunnel bootstrap or LAN 直結** | 本書 (手動) |

## 1. 前提と用意するもの

- 2 台 (以後 `Alice` / `Bob` と呼ぶ)
- 各 PC に: Rust stable / Node 22+ / cmake (audio test を行う場合)
- Cernere は **未起動でも可** (Susurrus は v0.0 では Cernere token を必須にしていない、 暫定 user URI = `cr:local-user`)
- Synergos daemon (`synergos-core`) を各 PC で起動できること
- 共通フォーラム名 (例: `home/family`) を事前合意

> **memory ref**: Synergos 側の運用上の罠は [`E:/Document/Ars/Synergos/OPERATIONAL-TEST.md`](../Synergos/OPERATIONAL-TEST.md) と memory `project_synergos.md` も参照。

### 1.1 ネットワーク経路の選択

| 経路 | 要件 | 備考 |
|------|------|------|
| LAN 直結 | 同一サブネット | 一番楽。 mDNS は使わないので IP 直指定 |
| Cloudflare Tunnel + 直結 QUIC | ドメイン + cloudflared | NAT 越え本番経路。 HTTPS bootstrap + UDP/7777 直結 |
| Tailscale | 各 PC に Tailscale | 一番安定。 GPS / 長期 WS と同経路 (memory: `feedback_sensitive_data_via_tailscale.md`) |

最初は LAN または Tailscale を推奨。 Cloudflare Tunnel 経路は別途 Synergos の OPERATIONAL-TEST に手順あり。

## 2. ビルド (各 PC で 1 度ずつ)

```bash
git clone https://github.com/LUDIARS/Susurrus
cd Susurrus/susurrus-tauri/frontend && npm ci && npm run build
cd ../.. && cargo build -p susurrus-tauri --release
# audio (Spatial) を試す場合
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo build -p susurrus-tauri --release --features audio
```

`susurrus-tauri.exe` (`/.so`) が `target/release/` に出る。 別途 Synergos も clone してビルド:

```bash
git clone https://github.com/LUDIARS/Synergos
cd Synergos && cargo build --release -p synergos-core
```

## 3. 各 PC で daemon を立ち上げる

### 3.1 Synergos daemon (各 PC)

```bash
# Alice
./target/release/synergos-core daemon --identity "$HOME/.synergos/identity.alice.toml"
# Bob (別ホスト)
./target/release/synergos-core daemon --identity "$HOME/.synergos/identity.bob.toml"
```

`identity` は ed25519 キーで、 PC 毎に別物。 確認:

```bash
synergos-core peer list <project_id>     # まだ peer は居ない (こちら側 daemon 内のみ)
```

### 3.2 互いの peer を登録 (LAN 直結 ver)

```bash
# Alice 側で Bob の advertise URL を取得
synergos-core peer-info-url
# → http://192.168.10.21:7780/peer-info みたいなやつ

# Bob 側で:
synergos-core peer add-by-url <Alice の URL>

# Alice 側でも Bob を登録 (双方向)
synergos-core peer add-by-url <Bob の URL>
```

### 3.3 Susurrus を Synergos 接続モードで起動 (各 PC)

Synergos daemon が動いている状態で:

```bash
SUSURRUS_USER=cr:alice SUSURRUS_SYNERGOS=1 \
  ./target/release/susurrus-tauri
```

(Bob 側は `cr:bob` に置換)

GUI が立ち上がる。 初回 `SetupScreen` で `home/family` forum + `general` channel を作成。 Alice 側だけ作ればよく、 Bob 側は **後で chain auto-pull 経由で同期される** ことを確認するのが目的。

## 4. 検証チェックリスト (#3 本体)

### 4.1 SLEEP 経路: forum / channel が auto-pull で届く

- [ ] **Alice**: `home/family` forum + `general` channel + 1 thread (例: "テスト")を作成
- [ ] **Bob**: 数秒後、 forum tree に `home/family/general` が現れ、 thread "テスト" が表示
- [ ] Bob の `$SUSURRUS_DATA/forums/home/family/_forum.md` がファイルとして存在する
- [ ] Alice / Bob 双方の SQLite で `forum / channel / thread / reply` の行数が一致

検証コマンド (Bob 側):

```bash
sqlite3 $SUSURRUS_DATA/db/susurrus.db \
  "SELECT (SELECT COUNT(*) FROM forum), (SELECT COUNT(*) FROM channel), (SELECT COUNT(*) FROM thread)"
```

### 4.2 SLEEP 経路: 投稿 / 返信が往復する

- [ ] Bob → "返事" を thread "テスト" に reply
- [ ] Alice 側に 5-10 秒以内で reply が現れる
- [ ] thread の `last_reply_ts` / `reply_count` が両者で揃う
- [ ] Bob で送った reply の md (例: `forums/home/family/general/t_*/m_*.md`) が Alice 側にもファイル存在

### 4.3 ACTIVE 経路: typing indicator がリアルタイムで届く

- [ ] Alice、 thread を開いて composer に入力中
- [ ] Bob 側 UI に "Alice が入力中…" の表示が **2 秒以内** に出る
- [ ] Alice が入力をやめて 3 秒後に消える

ログで `SUT1` magic の送受信を観察すれば下層も確認できる:

```bash
RUST_LOG=susurrus_synergos=trace,susurrus_rt=trace ./susurrus-tauri
```

### 4.4 ACTIVE 経路: presence ping (RTT) が更新される

- [ ] `sqlite3 .../susurrus.db "SELECT user_uri, state, rtt_ms, last_seen FROM presence"`
- [ ] Bob 側で Alice の presence が `state = active` + `rtt_ms` が現実的な値 (LAN なら数 ms、 WAN なら数十 ms)

### 4.5 (任意) Spatial Chat — 音声 + 位置

`--features audio` で起動した場合のみ。

- [ ] Alice / Bob 双方が SDK 経由で `report_position` を 1Hz で送信する mock app (例: 簡易 CLI) を起動
- [ ] Alice が話す → Bob 側で Alice の声が **位置に応じた音量 / 左右 pan** で聞こえる
- [ ] Alice が遠ざかる → Bob で音が小さくなる (linear, max=10m 想定)

mock CLI 例 (susurrus-sdk から):

```rust
let s = susurrus_sdk::Susurrus::local_default();
loop {
    s.report_position(
        "cr:alice", "<forum_id>",
        SpatialPosition { x: 0.0, y: 0.0, z: 0.0, ..Default::default() },
    ).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

## 5. 既知のハマりどころ (memory より)

| 症状 | 対処 |
|------|------|
| Win 側からの IPv4 destination 送信失敗 | Susurrus は Synergos に乗っているので影響を受ける。 Synergos `quic.bind = "0.0.0.0:0"` を確認 (memory `feedback_synergos_windows_dualstack_quic.md`) |
| LAN 越しに peer 接続が成立しない | OS firewall / antivirus で UDP/7777 ブロックされていないか |
| Cloudflare Tunnel 経路が ERR_SSL_VERSION_OR_CIPHER_MISMATCH | universal SSL は 1 階層 wildcard まで (memory `feedback_cloudflare_universal_ssl_depth.md`) |
| `9001` で Imperativus / MinIO が衝突 | Susurrus は 17370 だが Synergos の関連 daemon が居ると port 競合に注意 |
| Susurrus port 17370 が他で使われている | `SUSURRUS_LOCAL_PORT=17371` 等で逃がす |

## 6. 期待値 / SLA

- SLEEP 経路の片道伝搬時間: 5-10 秒 (Synergos gossip + auto-pull のラウンドトリップ)
- ACTIVE 経路の typing 反映: 1-2 秒
- presence の SLA は無し (5-10 秒で `last_seen` が更新されれば OK)

## 7. テスト終了時の cleanup

```bash
# 各 PC で:
$SUSURRUS_DATA を削除すれば forum / SQLite / typing 状態は完全リセット
synergos-core daemon の identity は残しておくと再 peer 登録が省略できる
```

## 8. CI #1 と本書 #3 の関係

- #1 (CI smoke): forum CRUD + HTTP API が「単 1 プロセス内で動く」 ことの保証
- #3 (本書): 「**2 PC で実際に喋れる**」 ことの保証 — manual で各リリース前に走らせる
- 将来 #3 を CI 化したい場合は 2 self-hosted runner + Synergos test cluster が要る。 v1.x では未着手。
