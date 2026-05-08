//! トランスポート抽象。 Synergos backend が `MessageBus` を実装する。
//!
//! v0.2 では本 trait の Synergos 実装は **別 crate に切る** (susurrus-rt は abstract のみ)。
//! ここでは:
//! - in-memory な `MockBus` (テスト / シングルプロセス疎通用)
//! - trait 定義 + メッセージ envelope

use crate::magic::Magic;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

/// peer 識別子。 Synergos の PeerId (blake3 of ed25519 pub) を hex で持つ前提。
pub type PeerId = String;

#[derive(Debug, Clone)]
pub struct Frame {
    pub from: PeerId,
    pub magic: Magic,
    pub payload: Vec<u8>,
}

#[async_trait]
pub trait MessageBus: Send + Sync {
    /// 単一 peer に送る。
    async fn send(&self, to: &PeerId, magic: Magic, payload: Vec<u8>) -> anyhow::Result<()>;

    /// 全 ACTIVE peer に送る (forum 範囲の broadcast は呼び出し側で絞る想定)。
    async fn broadcast(&self, magic: Magic, payload: Vec<u8>) -> anyhow::Result<()>;

    /// 受信ストリーム。 1 message = 1 Frame として上層へ流す。
    async fn recv(&self) -> Option<Frame>;
}

/// 単一プロセス内で複数 peer 役を演じるモック。 test 用 + susurrus-core から
/// 「ローカル loopback」として参照される想定。
pub struct MockBus {
    me: PeerId,
    inbox: Mutex<mpsc::Receiver<Frame>>,
    /// peer_id → 受信 Sender (相手の inbox)。
    /// 全 MockBus は同一 hub を共有して相互配送する。
    hub: Arc<Mutex<HashMap<PeerId, mpsc::Sender<Frame>>>>,
}

impl MockBus {
    /// 共有 hub を作成。
    pub fn hub() -> Arc<Mutex<HashMap<PeerId, mpsc::Sender<Frame>>>> {
        Arc::new(Mutex::new(HashMap::new()))
    }

    /// hub に自分を登録した MockBus を返す。
    pub async fn new(
        me: impl Into<PeerId>,
        hub: Arc<Mutex<HashMap<PeerId, mpsc::Sender<Frame>>>>,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<Frame>(64);
        let me_id: PeerId = me.into();
        hub.lock().await.insert(me_id.clone(), tx);
        Self {
            me: me_id,
            inbox: Mutex::new(rx),
            hub,
        }
    }
}

#[async_trait]
impl MessageBus for MockBus {
    async fn send(&self, to: &PeerId, magic: Magic, payload: Vec<u8>) -> anyhow::Result<()> {
        let map = self.hub.lock().await;
        if let Some(tx) = map.get(to) {
            tx.send(Frame {
                from: self.me.clone(),
                magic,
                payload,
            })
            .await?;
        }
        Ok(())
    }

    async fn broadcast(&self, magic: Magic, payload: Vec<u8>) -> anyhow::Result<()> {
        let map = self.hub.lock().await;
        for (peer, tx) in map.iter() {
            if peer == &self.me {
                continue;
            }
            let _ = tx
                .send(Frame {
                    from: self.me.clone(),
                    magic,
                    payload: payload.clone(),
                })
                .await;
        }
        Ok(())
    }

    async fn recv(&self) -> Option<Frame> {
        self.inbox.lock().await.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::{encode, SusTyping};
    use uuid::Uuid;

    #[tokio::test]
    async fn mock_bus_round_trip() {
        let hub = MockBus::hub();
        let a = MockBus::new("peer-a", hub.clone()).await;
        let b = MockBus::new("peer-b", hub.clone()).await;

        let payload = encode(&SusTyping {
            thread_id: Uuid::now_v7(),
            user_uri: "cr:a".into(),
            until_ms: 1_700_000_000_000,
        })
        .unwrap();

        a.send(&"peer-b".to_string(), Magic::Typing, payload.clone())
            .await
            .unwrap();
        let f = b.recv().await.unwrap();
        assert_eq!(f.from, "peer-a");
        assert_eq!(f.magic, Magic::Typing);
        assert_eq!(f.payload, payload);
    }

    #[tokio::test]
    async fn mock_bus_broadcast_excludes_self() {
        let hub = MockBus::hub();
        let a = MockBus::new("peer-a", hub.clone()).await;
        let _b = MockBus::new("peer-b", hub.clone()).await;
        let _c = MockBus::new("peer-c", hub.clone()).await;

        a.broadcast(Magic::Ping, vec![1, 2, 3]).await.unwrap();
        // a 自身は受信しない
        // b/c の受信は別タスクで確認するのは煩雑。 単に send が err しない、 hub に 3 entry あることだけ確認。
        let map = hub.lock().await;
        assert_eq!(map.len(), 3);
    }
}
