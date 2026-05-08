//! Susurrus Markdown schema (frontmatter + content) の parser/writer。
//!
//! 仕様: ../../spec/MD-SCHEMA.md

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum MdError {
    #[error("frontmatter missing or malformed")]
    FrontmatterMissing,
    #[error("kind '{0}' is not recognized")]
    UnknownKind(String),
    #[error("required field '{0}' missing for kind '{1}'")]
    MissingField(&'static str, &'static str),
    #[error("path mismatch: frontmatter says {expected:?}, file at {actual:?}")]
    PathMismatch { expected: String, actual: PathBuf },
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// 共通フィールドだけ最初に判定するための kind tag。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Forum,
    Channel,
    Thread,
    Reply,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FrontMatter {
    Forum(ForumMeta),
    Channel(ChannelMeta),
    Thread(ThreadMeta),
    Reply(ReplyMeta),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForumMeta {
    pub id: Uuid,
    pub path: String,
    pub name: String,
    #[serde(default)]
    pub parent: Option<String>,
    pub visibility: String,
    #[serde(default)]
    pub group: Option<String>,
    pub created_at: DateTime<FixedOffset>,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMeta {
    pub id: Uuid,
    pub forum: Uuid,
    pub path: String,
    pub name: String,
    #[serde(default)]
    pub topic: String,
    #[serde(default = "default_sort")]
    pub sort: i32,
    pub created_at: DateTime<FixedOffset>,
    pub created_by: String,
    #[serde(default)]
    pub archived: bool,
}

fn default_sort() -> i32 { 100 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadMeta {
    pub id: Uuid,
    pub channel: Uuid,
    pub forum: Uuid,
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub author: String,
    pub ts: DateTime<FixedOffset>,
    #[serde(default)]
    pub edited_at: Option<DateTime<FixedOffset>>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplyMeta {
    pub id: Uuid,
    pub thread: Uuid,
    pub parent: Uuid,
    pub forum: Uuid,
    pub channel: Uuid,
    pub author: String,
    pub ts: DateTime<FixedOffset>,
    #[serde(default)]
    pub edited_at: Option<DateTime<FixedOffset>>,
    #[serde(default)]
    pub deleted: bool,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub mentions: Vec<String>,
    #[serde(default)]
    pub reactions: std::collections::BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub kind: String,
    pub cid: String,
    pub name: String,
}

/// 1 つの md ファイルを (frontmatter, body) に分けて読む。
pub fn parse(input: &str) -> Result<(FrontMatter, String), MdError> {
    let matter = gray_matter::Matter::<gray_matter::engine::YAML>::new();
    let result = matter.parse(input);
    let data = result.data.ok_or(MdError::FrontmatterMissing)?;
    // gray_matter::Pod -> serde_yaml::Value 経由で typed に
    let yaml: serde_yaml::Value = serde_yaml::from_str(
        &serde_yaml::to_string(&data)
            .map_err(MdError::Yaml)?
    ).map_err(MdError::Yaml)?;
    let fm: FrontMatter = serde_yaml::from_value(yaml)?;
    Ok((fm, result.content))
}

/// (frontmatter, body) を md 文字列にする。
pub fn serialize(fm: &FrontMatter, body: &str) -> Result<String, MdError> {
    let yaml = serde_yaml::to_string(fm)?;
    Ok(format!("---\n{}---\n{}", yaml, body))
}

/// 内容ハッシュ (md_hash カラム用)。
pub fn hash(input: &str) -> String {
    blake3::hash(input.as_bytes()).to_hex().to_string()
}
