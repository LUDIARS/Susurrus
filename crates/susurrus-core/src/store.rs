//! md ファイル ↔ SQLite の同期。

use std::path::{Path, PathBuf};

pub struct MdStore {
    pub forum_root: PathBuf,
}

impl MdStore {
    pub fn new(forum_root: impl Into<PathBuf>) -> Self {
        Self { forum_root: forum_root.into() }
    }

    /// forum_root を walk して md を全列挙。 v0.1 で実装。
    pub fn walk(&self) -> impl Iterator<Item = PathBuf> {
        std::iter::empty::<PathBuf>()
    }

    /// reply のファイル path を組み立てる:
    /// `<forum_root>/<channel.path>/t_<date>_<short>/m_<short>.md`
    pub fn reply_path(&self, _channel_path: &str, _thread_short: &str, _reply_short: &str) -> PathBuf {
        unimplemented!()
    }
}

/// FS watcher (notify) は v0.1 で導入。
pub fn watch(_path: &Path) {
    // TODO: notify crate
}
