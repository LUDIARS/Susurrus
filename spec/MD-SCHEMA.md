# Susurrus Markdown スキーマ

> 全データの正本。 frontmatter で帰属を表現し、 SQLite はあくまでキャッシュ。

## 0. 共通ルール

- 改行: LF
- エンコーディング: UTF-8 (BOM なし)
- frontmatter: YAML、 `---` で挟む
- 拡張子: `.md`
- パスはすべて `forum_root` (= `$SUSURRUS_DATA/forums`) からの相対 path

## 1. forum メタ (`<forum>/_forum.md`)

forum (= Discord guild + forum 統合体) のメタデータ。

```markdown
---
kind: forum
id: f_01HK8N3M...                 # UUIDv7
path: work/ludiars                # forum_root からの path
name: LUDIARS Workspace
parent: work                      # 上位 forum (省略可、 root forum では null)
visibility: cernere-group         # public | cernere-group | invite-only
group: cg_4f2a...                 # cernere-group のとき必須
created_at: 2026-05-08T12:00:00+09:00
created_by: cr:user-uuid
---
（任意の説明文。 Markdown）
```

- `path` は **ファイルシステム上の path と一致** させる。 不一致は core daemon が起動時に検出して警告。
- `id` は UUIDv7 (時系列ソート可能)。 frontmatter で永久不変。

## 2. channel メタ (`<forum>/<channel>/_channel.md`)

channel (= Discord channel)。 forum 直下に複数置ける。

```markdown
---
kind: channel
id: c_01HK8N4...
forum: f_01HK8N3M...              # 親 forum の id
path: work/ludiars/general
name: general
topic: 雑談用
sort: 100                         # 表示順 (小さいほど上)
created_at: 2026-05-08T12:01:00+09:00
created_by: cr:user-uuid
archived: false
---
```

## 3. thread root (`<channel>/t_<yyyy-mm-dd>_<short>.md`)

thread の起点メッセージ (Slack thread root / Discord forum post)。

ファイル名規則: `t_<日付>_<short-id>.md`
- `<日付>` = `created_at` の日 (YYYY-MM-DD)
- `<short-id>` = thread の `id` (UUIDv7) の最後 4 hex (重複時はもう 4 hex 追加)

```markdown
---
kind: thread
id: t_01HK8N5...
channel: c_01HK8N4...
forum: f_01HK8N3M...
title: Susurrus 設計レビュー
tags: [design, chat]
author: cr:user-uuid
ts: 2026-05-08T12:05:00+09:00
edited_at: null
pinned: false
locked: false
---
本文 (Markdown)
```

## 4. reply (`<channel>/t_<...>/m_<short>.md`)

thread に紐づく返信。 thread root の同名サブディレクトリに置く。

ファイル名規則: `m_<short-id>.md` (`<short-id>` は thread root と同じ規則)

```markdown
---
kind: reply
id: m_01HK8N6...
thread: t_01HK8N5...
parent: t_01HK8N5...              # 直接の返信先 (thread root か別 reply)
forum: f_01HK8N3M...              # 検索高速化用 denormalize
channel: c_01HK8N4...             # 同上
author: cr:user-uuid
ts: 2026-05-08T12:06:00+09:00
edited_at: null
attachments:
  - kind: image
    cid: blake3:abcd...           # Synergos CID
    name: screenshot.png
mentions:
  - cr:other-user-uuid
reactions:
  "👍": [cr:user-a, cr:user-b]
---
本文 (Markdown)

> 引用は通常の Markdown blockquote
```

### 返信ツリー

- `parent` で thread root (`t_*`) または別 reply (`m_*`) を指す。 これにより X 風のリプライチェーン (枝分かれ) を表現できる。
- thread 全体に対する返信 (= フラット) は `parent = thread.id`。
- 削除は **frontmatter `deleted: true` + 本文を空にする** (履歴は Synergos chain に残るため、ファイルは残す)。

## 5. ID / Path 命名

- `f_*` forum
- `c_*` channel
- `t_*` thread root
- `m_*` reply
- すべて UUIDv7 を base32 / base58 で encode (実装で確定)
- frontmatter の `id` は完全形、 ファイル名 / 短縮表示は末尾 4-8 文字

## 6. DM の特殊扱い

- forum path は `_dm/<sorted-user-pair-hash>`
- `kind: forum` の forum に `visibility: dm` を追加
- channel は 1 つだけ (`general`) を自動生成
- それ以外は通常のスレッド構造と同じ

## 7. 添付ファイル

- 画像 / 動画 / バイナリは **Synergos の CID (blake3)** で参照。
- 実体は Synergos の content store に置く。 Susurrus は md frontmatter に `cid` を書くだけ。
- これにより Synergos auto-pull で peer 間に自動配信される。

## 8. 互換性 / バージョニング

- frontmatter に `schema: 1` を将来追加可能 (現在は省略 = 1 とみなす)。
- breaking change は `schema: 2` を introduce + migration スクリプトを `tools/migrate-md/` に置く。
- 古い schema を読む側は warning + best-effort parse。

## 9. 不変条件 (core daemon が検証)

1. `path` (frontmatter) と FS 上の path が一致
2. `id` がワークスペース内で一意
3. `parent` / `thread` / `channel` / `forum` の参照先が存在
4. `kind` ごとに必須フィールドが揃っている
5. `ts` が monotonic でなくても良い (clock skew 許容)、 ただし thread 内の sort は (ts, id) tuple で安定化

検証失敗時は SQLite に warning row を追加 + GUI に表示するが **ファイル自身は触らない** (md がドキュメントとして手編集される可能性を尊重)。
