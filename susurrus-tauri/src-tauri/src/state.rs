use parking_lot::Mutex;
use std::path::Path;
use susurrus_core::{db::Db, store::MdStore};

pub struct AppState {
    pub inner: Mutex<Inner>,
}

pub struct Inner {
    pub db: Db,
    pub store: MdStore,
}

impl AppState {
    pub fn open(data_dir: &Path) -> anyhow::Result<Self> {
        let forum_root = data_dir.join("forums");
        let db_path = data_dir.join("db").join("susurrus.db");
        std::fs::create_dir_all(&forum_root)?;
        let db = Db::open(&db_path)?;
        let store = MdStore::new(&forum_root);
        Ok(Self {
            inner: Mutex::new(Inner { db, store }),
        })
    }
}
