//! `SynergosBackend` trait と 2 つの実装:
//! - [`NoopBackend`] = 何もしない (Synergos 未起動時の fallback)
//! - [`IpcBackend`] = synergos-ipc::IpcClient を使う本実装
//!
//! どちらも `SynergosBridge` から `Arc<dyn SynergosBackend>` で受ける。

use crate::bridge::IncomingFile;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

#[async_trait]
pub trait SynergosBackend: Send + Sync {
    /// project_id で Synergos network に参加。 idempotent な実装が望ましい。
    async fn project_open(
        &self,
        project_id: &str,
        root_path: &Path,
        display_name: Option<&str>,
    ) -> anyhow::Result<()>;

    /// md ファイル変更を chain に commit。
    async fn publish_update(
        &self,
        project_id: &str,
        files: &[PathBuf],
    ) -> anyhow::Result<()>;

    /// 受信ファイル通知を受け取る Receiver を返す。 同じ backend に対して 1 度だけ呼ぶ前提
    /// (それ以外の場合の挙動は実装依存)。
    fn incoming_files_receiver(&self) -> mpsc::Receiver<IncomingFile>;
}

// ──────────────────────────────────────────────────────────────────
// NoopBackend
// ──────────────────────────────────────────────────────────────────

pub struct NoopBackend {
    rx: Mutex<Option<mpsc::Receiver<IncomingFile>>>,
}

impl Default for NoopBackend {
    fn default() -> Self {
        let (_tx, rx) = mpsc::channel::<IncomingFile>(1);
        Self { rx: Mutex::new(Some(rx)) }
    }
}

impl NoopBackend {
    pub fn new() -> Self { Self::default() }
    pub fn arc() -> Arc<dyn SynergosBackend> { Arc::new(Self::default()) }
}

#[async_trait]
impl SynergosBackend for NoopBackend {
    async fn project_open(
        &self,
        _project_id: &str,
        _root_path: &Path,
        _display_name: Option<&str>,
    ) -> anyhow::Result<()> {
        tracing::info!("synergos backend: noop (project_open ignored)");
        Ok(())
    }

    async fn publish_update(
        &self,
        _project_id: &str,
        files: &[PathBuf],
    ) -> anyhow::Result<()> {
        tracing::info!("synergos backend: noop publish_update ({} files)", files.len());
        Ok(())
    }

    fn incoming_files_receiver(&self) -> mpsc::Receiver<IncomingFile> {
        // 1 度だけ取り出せる receiver (Noop なので常に空の受信)
        let mut guard = futures_block_on(self.rx.lock());
        guard.take().unwrap_or_else(|| {
            let (_tx, rx) = mpsc::channel::<IncomingFile>(1);
            rx
        })
    }
}

// ──────────────────────────────────────────────────────────────────
// IpcBackend
// ──────────────────────────────────────────────────────────────────

pub struct IpcBackend {
    /// synergos-ipc::IpcClient ラッパ。 Mutex で送信を排他。
    client: Arc<Mutex<synergos_ipc::IpcClient>>,
    /// TransferCompleted を流すための tx。 spawn された listener task が push する。
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingFile>>>,
}

impl IpcBackend {
    /// IpcClient を新規接続して構築。 listener task も同時に spawn。
    pub async fn connect() -> anyhow::Result<Arc<Self>> {
        let mut client = synergos_ipc::IpcClient::connect()
            .await
            .map_err(|e| anyhow::anyhow!("synergos ipc connect failed: {e}"))?;

        // event subscribe
        let _ = client
            .send(synergos_ipc::IpcCommand::Subscribe {
                events: vec![synergos_ipc::event::EventFilter::All],
            })
            .await;

        let (tx, rx) = mpsc::channel::<IncomingFile>(64);

        // event listener
        tokio::spawn(async move {
            // NOTE: client move out — but we just took it above. We need a different
            // arrangement: keep client in Arc<Mutex>, spawn a separate event loop.
            // 簡略化: IpcClient::recv_event は &mut self を要求するため、
            // 本実装では client を分割して持たせるか、あるいは cloned channel から
            // 拾う設計にする必要がある。 v0.3 では「初回 connect 時の listener は
            // 起動し、その後別 client を connect して send 用に使い分ける」 経路を取る。
            // ここでは listener なしの空 future にし、 Synergos 側拡張完了後に再設計。
            let _ = tx; // 抑制
        });

        Ok(Arc::new(Self {
            client: Arc::new(Mutex::new(client)),
            incoming_rx: Mutex::new(Some(rx)),
        }))
    }
}

#[async_trait]
impl SynergosBackend for IpcBackend {
    async fn project_open(
        &self,
        project_id: &str,
        root_path: &Path,
        display_name: Option<&str>,
    ) -> anyhow::Result<()> {
        let cmd = synergos_ipc::IpcCommand::ProjectOpen {
            project_id: project_id.to_string(),
            root_path: root_path.to_path_buf(),
            display_name: display_name.map(|s| s.to_string()),
        };
        let mut g = self.client.lock().await;
        let resp = g.send(cmd).await
            .map_err(|e| anyhow::anyhow!("synergos send: {e}"))?;
        check_ok(resp, "ProjectOpen")
    }

    async fn publish_update(
        &self,
        project_id: &str,
        files: &[PathBuf],
    ) -> anyhow::Result<()> {
        let cmd = synergos_ipc::IpcCommand::PublishUpdate {
            project_id: project_id.to_string(),
            file_paths: files.to_vec(),
        };
        let mut g = self.client.lock().await;
        let resp = g.send(cmd).await
            .map_err(|e| anyhow::anyhow!("synergos send: {e}"))?;
        check_ok(resp, "PublishUpdate")
    }

    fn incoming_files_receiver(&self) -> mpsc::Receiver<IncomingFile> {
        let mut guard = futures_block_on(self.incoming_rx.lock());
        guard.take().unwrap_or_else(|| {
            let (_tx, rx) = mpsc::channel(1);
            rx
        })
    }
}

// 同期 context (trait method の中) で Mutex::lock() するためのヘルパ。
// 将来的に trait 全体を async にして取り除く。
fn futures_block_on<F: std::future::Future>(f: F) -> F::Output {
    // tokio::runtime::Handle::current で blocking 経由で進める手もあるが、
    // 現状は Mutex の中身を取るだけなので busy spin は要らない。
    // try_lock で十分だが、 v0.3 では同期化を最小限に。
    futures_lite::future::block_on(f)
}

fn check_ok(resp: synergos_ipc::IpcResponse, label: &str) -> anyhow::Result<()> {
    match resp {
        synergos_ipc::IpcResponse::Ok => Ok(()),
        synergos_ipc::IpcResponse::Error { code, message } => {
            Err(anyhow::anyhow!("synergos {label} error code={code}: {message}"))
        }
        other => Err(anyhow::anyhow!("synergos {label} unexpected response: {other:?}")),
    }
}
