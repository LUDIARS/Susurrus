//! md ファイルの walk + 補助関数。 SQLite との同期は indexer モジュール担当。

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct MdStore {
    pub forum_root: PathBuf,
}

impl MdStore {
    pub fn new(forum_root: impl Into<PathBuf>) -> Self {
        Self { forum_root: forum_root.into() }
    }

    /// forum_root を walk して `.md` ファイルを全列挙。
    pub fn walk(&self) -> impl Iterator<Item = PathBuf> + '_ {
        WalkDir::new(&self.forum_root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.into_path())
            .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("md"))
    }

    /// forum_root からの相対 path を返す。
    pub fn rel(&self, p: &Path) -> Option<PathBuf> {
        p.strip_prefix(&self.forum_root).ok().map(|p| p.to_path_buf())
    }
}

/// FS 上の path を forum/channel/thread/reply の論理 path 文字列に変換する補助。
/// path セパレータを `/` に正規化する。
pub fn norm_path(p: &Path) -> String {
    p.iter()
        .filter_map(|s| s.to_str())
        .collect::<Vec<_>>()
        .join("/")
}
