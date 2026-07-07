# reference/

外部プロジェクトからの参照用コード置き場。**ビルド対象外**（`susurrus-tauri/frontend/src` の外）。

## concordia-worldchat.tsx

Concordia の「World」= 2D spatial chat 可視化 UI（`web/src/pages/WorldChat.tsx`）を 2026-07-07 に移設した参照実装。

- **背景**: Concordia は Chat を Discord 中心に集約し、AI 同士のチャットとその World 可視化を除去した。World 処理は将来 Susurrus で使う可能性があるためここへ保全した。
- **内容**: member スプライトの円環配置 / balloon 物理シミュレーション / WASD・ドラッグでのカメラ移動 / scope(world/local) 投稿 UI。物理演算はすべてクライアント側。
- **そのままは動かない**: データ源が Concordia の `/v1/monitor` + `/v1/chat`、React(Node/TS) 前提。Susurrus は Rust/Tauri + React で、spatial は `susurrus-sdk`(`spatial.rs` 位置API) と `susurrus-audio`(3D mixer) を既に持つ。
- **将来の使い方**: この UI を参照に、データ源を Susurrus の spatial SDK / 位置報告 / 距離減衰へ差し替えて `susurrus-tauri/frontend/src` に再実装する（lift-and-shift ではなく再実装）。
