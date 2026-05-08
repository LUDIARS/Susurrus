use parking_lot::Mutex;
use std::path::Path;
use std::sync::Arc;
use susurrus_core::{db::Db, store::MdStore};
use susurrus_memoria::MemoriaClient;
use susurrus_rt::typing::TypingTracker;
use susurrus_synergos::{NoopBackend, SynergosBackend, SynergosBridge, SynergosConfig};

pub struct AppState {
    pub inner: Mutex<Inner>,
    /// Synergos bridge — SLEEP 経路 (chain commit + auto-pull)。 default = Noop。
    pub synergos: Arc<SynergosBridge>,
    /// Memoria クライアント (opt-out 可、 enabled が false のときは早期 error)。
    pub memoria: Arc<MemoriaClient>,
}

pub struct Inner {
    pub db: Db,
    pub store: MdStore,
    pub typing: TypingTracker,
    pub current_user: String,
}

impl AppState {
    pub fn open(data_dir: &Path) -> anyhow::Result<Self> {
        let forum_root = data_dir.join("forums");
        let db_path = data_dir.join("db").join("susurrus.db");
        std::fs::create_dir_all(&forum_root)?;
        let db = Db::open(&db_path)?;
        let store = MdStore::new(&forum_root);
        let current_user =
            std::env::var("SUSURRUS_USER").unwrap_or_else(|_| "cr:local-user".into());

        // Synergos backend
        let backend: Arc<dyn SynergosBackend> =
            if std::env::var("SUSURRUS_SYNERGOS").ok().as_deref() == Some("1") {
                // 失敗しても Noop に fallback (起動継続を優先)
                match futures_lite::future::block_on(
                    susurrus_synergos::backend::IpcBackend::connect(),
                ) {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::warn!("synergos ipc connect failed, falling back to Noop: {e:#}");
                        NoopBackend::arc()
                    }
                }
            } else {
                NoopBackend::arc()
            };
        let cfg = SynergosConfig {
            project_id: "susurrus".into(),
            root_path: forum_root.clone(),
            display_name: Some("Susurrus".into()),
        };
        let synergos = Arc::new(SynergosBridge::new(cfg, backend));
        let _ = futures_lite::future::block_on(synergos.open_project());

        // Memoria
        let memoria_endpoint = std::env::var("SUSURRUS_MEMORIA_ENDPOINT")
            .unwrap_or_else(|_| "http://127.0.0.1:5180".into());
        let memoria_token = std::env::var("SUSURRUS_MEMORIA_TOKEN").ok();
        let memoria_enabled = std::env::var("SUSURRUS_MEMORIA_DISABLED").is_err();
        let memoria = Arc::new(MemoriaClient::new(
            memoria_endpoint,
            memoria_token,
            memoria_enabled,
        ));

        Ok(Self {
            inner: Mutex::new(Inner {
                db,
                store,
                typing: TypingTracker::new(),
                current_user,
            }),
            synergos,
            memoria,
        })
    }
}
