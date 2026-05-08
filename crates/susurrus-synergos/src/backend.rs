//! `SynergosBackend` trait と 2 つの実装:
//! - [`NoopBackend`] = 何もしない (Synergos 未起動時の fallback)
//! - [`IpcBackend`] = synergos-ipc::IpcClient を使う本実装。
//!   送信用と event 受信用の 2 接続を別々に張る (recv_event は &mut self を要求するため)。

use crate::bridge::IncomingFile;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use susurrus_rt::magic::Magic;
use susurrus_rt::transport::{Frame, MessageBus, PeerId};
use tokio::sync::{mpsc, Mutex};

#[async_trait]
pub trait SynergosBackend: Send + Sync {
    async fn project_open(
        &self,
        project_id: &str,
        root_path: &Path,
        display_name: Option<&str>,
    ) -> anyhow::Result<()>;

    async fn publish_update(&self, project_id: &str, files: &[PathBuf]) -> anyhow::Result<()>;

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
        Self {
            rx: Mutex::new(Some(rx)),
        }
    }
}

impl NoopBackend {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn arc() -> Arc<dyn SynergosBackend> {
        Arc::new(Self::default())
    }
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

    async fn publish_update(&self, _project_id: &str, files: &[PathBuf]) -> anyhow::Result<()> {
        tracing::info!(
            "synergos backend: noop publish_update ({} files)",
            files.len()
        );
        Ok(())
    }

    fn incoming_files_receiver(&self) -> mpsc::Receiver<IncomingFile> {
        let mut guard = futures_block_on(self.rx.lock());
        guard.take().unwrap_or_else(|| {
            let (_tx, rx) = mpsc::channel::<IncomingFile>(1);
            rx
        })
    }
}

// ──────────────────────────────────────────────────────────────────
// IpcBackend (本実装)
// ──────────────────────────────────────────────────────────────────

pub struct IpcBackend {
    /// 送信用 IpcClient。 Mutex で送信を直列化。
    send_client: Arc<Mutex<synergos_ipc::IpcClient>>,
    /// IncomingFile receiver (TransferCompleted 経路、 v0.3 では Noop 同等)。
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingFile>>>,
    /// PeerStreamReceived (= MessageBus 受信) を放出する。
    /// 1 度だけ取り出せる (= [`IpcBackend::take_message_bus`] が消費)。
    bus_rx: Mutex<Option<mpsc::Receiver<Frame>>>,
}

impl IpcBackend {
    /// IpcClient を 2 本接続して構築。 1 本は send 用、 もう 1 本は event 受信用。
    pub async fn connect() -> anyhow::Result<Arc<Self>> {
        let mut send_client = synergos_ipc::IpcClient::connect()
            .await
            .map_err(|e| anyhow::anyhow!("synergos ipc (send) connect failed: {e}"))?;
        let mut event_client = synergos_ipc::IpcClient::connect()
            .await
            .map_err(|e| anyhow::anyhow!("synergos ipc (event) connect failed: {e}"))?;

        // 送信側で event subscribe しても OK だが分担を明確化するため event 側で subscribe。
        let _ = event_client
            .send(synergos_ipc::IpcCommand::Subscribe {
                events: vec![synergos_ipc::event::EventFilter::All],
            })
            .await;
        // send 側でも no-op subscribe (Daemon が要求 ID を持っているため)
        let _ = send_client.send(synergos_ipc::IpcCommand::Ping).await;

        let (incoming_tx, incoming_rx) = mpsc::channel::<IncomingFile>(64);
        let (bus_tx, bus_rx) = mpsc::channel::<Frame>(256);

        // event listener task
        tokio::spawn(async move {
            loop {
                match event_client.recv_event().await {
                    Ok(synergos_ipc::IpcEvent::TransferCompleted {
                        peer_id, file_path, ..
                    }) => {
                        let abs = std::path::PathBuf::from(file_path);
                        let _ = incoming_tx
                            .send(IncomingFile {
                                peer_id,
                                abs_path: abs,
                            })
                            .await;
                    }
                    Ok(synergos_ipc::IpcEvent::PeerStreamReceived {
                        peer_id,
                        magic,
                        payload,
                    }) => {
                        if let Some(m) = Magic::from_bytes(&magic) {
                            let _ = bus_tx
                                .send(Frame {
                                    from: peer_id,
                                    magic: m,
                                    payload,
                                })
                                .await;
                        } else {
                            tracing::trace!(
                                "ignored unknown extension magic from synergos: {:?}",
                                magic
                            );
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!("synergos event listener exited: {e}");
                        break;
                    }
                }
            }
        });

        Ok(Arc::new(Self {
            send_client: Arc::new(Mutex::new(send_client)),
            incoming_rx: Mutex::new(Some(incoming_rx)),
            bus_rx: Mutex::new(Some(bus_rx)),
        }))
    }

    /// MessageBus 実装を取り出す。 1 度だけ呼べる。
    pub fn take_message_bus(self: &Arc<Self>) -> Option<SynergosBus> {
        let mut guard = futures_block_on(self.bus_rx.lock());
        let rx = guard.take()?;
        Some(SynergosBus {
            backend: self.clone(),
            inbox: Mutex::new(rx),
        })
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
        let mut g = self.send_client.lock().await;
        let resp = g
            .send(cmd)
            .await
            .map_err(|e| anyhow::anyhow!("synergos send: {e}"))?;
        check_ok(resp, "ProjectOpen")
    }

    async fn publish_update(&self, project_id: &str, files: &[PathBuf]) -> anyhow::Result<()> {
        let cmd = synergos_ipc::IpcCommand::PublishUpdate {
            project_id: project_id.to_string(),
            file_paths: files.to_vec(),
        };
        let mut g = self.send_client.lock().await;
        let resp = g
            .send(cmd)
            .await
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

// ──────────────────────────────────────────────────────────────────
// SynergosBus — MessageBus 実装
// ──────────────────────────────────────────────────────────────────

pub struct SynergosBus {
    backend: Arc<IpcBackend>,
    inbox: Mutex<mpsc::Receiver<Frame>>,
}

#[async_trait]
impl MessageBus for SynergosBus {
    async fn send(&self, to: &PeerId, magic: Magic, payload: Vec<u8>) -> anyhow::Result<()> {
        let cmd = synergos_ipc::IpcCommand::PeerSendStream {
            peer_id: to.clone(),
            magic: magic.bytes(),
            payload,
        };
        let mut g = self.backend.send_client.lock().await;
        let resp = g
            .send(cmd)
            .await
            .map_err(|e| anyhow::anyhow!("synergos send: {e}"))?;
        check_ok(resp, "PeerSendStream")
    }

    async fn broadcast(&self, magic: Magic, payload: Vec<u8>) -> anyhow::Result<()> {
        // Synergos には「全 peer broadcast」 IPC が無いので、 PeerList で peers を引いて
        // 順に PeerSendStream を投げる。 v0.3 では small forum の想定で簡易実装。
        let peers: Vec<String> = {
            let mut g = self.backend.send_client.lock().await;
            let resp = g
                .send(synergos_ipc::IpcCommand::PeerList {
                    project_id: "susurrus".into(),
                })
                .await
                .map_err(|e| anyhow::anyhow!("synergos PeerList: {e}"))?;
            match resp {
                synergos_ipc::IpcResponse::PeerList(peers) => {
                    peers.into_iter().map(|p| p.peer_id).collect()
                }
                other => return Err(anyhow::anyhow!("synergos PeerList unexpected: {other:?}")),
            }
        };
        for p in peers {
            // 失敗は warning で握り潰す (1 peer 失敗で broadcast 全停止しない)
            if let Err(e) = self.send(&p, magic, payload.clone()).await {
                tracing::warn!("synergos broadcast to {p} failed: {e:#}");
            }
        }
        Ok(())
    }

    async fn recv(&self) -> Option<Frame> {
        self.inbox.lock().await.recv().await
    }
}

// 同期 context (trait method の中) で Mutex::lock() するためのヘルパ。
fn futures_block_on<F: std::future::Future>(f: F) -> F::Output {
    futures_lite::future::block_on(f)
}

fn check_ok(resp: synergos_ipc::IpcResponse, label: &str) -> anyhow::Result<()> {
    match resp {
        synergos_ipc::IpcResponse::Ok => Ok(()),
        synergos_ipc::IpcResponse::Error { code, message } => Err(anyhow::anyhow!(
            "synergos {label} error code={code}: {message}"
        )),
        other => Err(anyhow::anyhow!(
            "synergos {label} unexpected response: {other:?}"
        )),
    }
}
