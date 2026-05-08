//! 上位層 (susurrus-core) が依存する API。 backend trait `SynergosBackend` を
//! 注入する形で、 テストでは Noop / モック、 本番では IpcClient を使う。

use crate::backend::SynergosBackend;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("ipc: {0}")]
    Ipc(String),
    #[error("backend: {0}")]
    Backend(#[from] anyhow::Error),
}

#[derive(Debug, Clone)]
pub struct SynergosConfig {
    /// Susurrus 用 project の論理 id (Synergos 側で 1 ユーザ 1 project 想定)
    pub project_id: String,
    /// project の root (= forum_root と同じパスを渡す前提)
    pub root_path: PathBuf,
    /// project 表示名
    pub display_name: Option<String>,
}

/// Synergos 側で TransferCompleted されたファイル (= 他 peer から届いた md)
#[derive(Debug, Clone)]
pub struct IncomingFile {
    pub peer_id: String,
    pub abs_path: PathBuf,
}

pub struct SynergosBridge {
    cfg: SynergosConfig,
    backend: Arc<dyn SynergosBackend>,
}

impl SynergosBridge {
    pub fn new(cfg: SynergosConfig, backend: Arc<dyn SynergosBackend>) -> Self {
        Self { cfg, backend }
    }

    pub fn config(&self) -> &SynergosConfig { &self.cfg }

    /// Susurrus project を open する (idempotent)。
    pub async fn open_project(&self) -> Result<(), BridgeError> {
        self.backend
            .project_open(&self.cfg.project_id, &self.cfg.root_path, self.cfg.display_name.as_deref())
            .await
            .map_err(BridgeError::Backend)
    }

    /// md ファイル群を chain に publish する (= 他 peer の auto-pull が起動)。
    pub async fn publish(&self, files: &[&Path]) -> Result<(), BridgeError> {
        let owned: Vec<PathBuf> = files.iter().map(|p| p.to_path_buf()).collect();
        self.backend
            .publish_update(&self.cfg.project_id, &owned)
            .await
            .map_err(BridgeError::Backend)
    }

    /// 受信イベントを mpsc 経由で配信する。 task は backend が spawn 済みである必要がある。
    pub fn subscribe_incoming(&self) -> mpsc::Receiver<IncomingFile> {
        self.backend.incoming_files_receiver()
    }
}
