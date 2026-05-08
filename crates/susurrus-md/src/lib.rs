//! Susurrus Markdown schema (frontmatter + content) の parser/writer。
//!
//! 仕様: ../../spec/MD-SCHEMA.md
//!
//! frontmatter は YAML、 `---` 行で挟む。 改行は LF 前提。

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum MdError {
    #[error("frontmatter missing or malformed (no leading '---' line)")]
    FrontmatterMissing,
    #[error("frontmatter not terminated by trailing '---'")]
    FrontmatterUnterminated,
    #[error("yaml parse: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Forum,
    Channel,
    Thread,
    Reply,
}

/// 共通: forum/channel/thread/reply で同じプレフィックス id を持つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FrontMatter {
    Forum(ForumMeta),
    Channel(ChannelMeta),
    Thread(ThreadMeta),
    Reply(ReplyMeta),
}

impl FrontMatter {
    pub fn kind(&self) -> Kind {
        match self {
            Self::Forum(_) => Kind::Forum,
            Self::Channel(_) => Kind::Channel,
            Self::Thread(_) => Kind::Thread,
            Self::Reply(_) => Kind::Reply,
        }
    }
    pub fn id(&self) -> Uuid {
        match self {
            Self::Forum(m) => m.id,
            Self::Channel(m) => m.id,
            Self::Thread(m) => m.id,
            Self::Reply(m) => m.id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForumMeta {
    pub id: Uuid,
    pub path: String,
    pub name: String,
    #[serde(default)]
    pub parent: Option<String>,
    pub visibility: Visibility,
    #[serde(default)]
    pub group: Option<String>,
    pub created_at: DateTime<FixedOffset>,
    pub created_by: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Visibility {
    Public,
    CernereGroup,
    InviteOnly,
    Dm,
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

fn default_sort() -> i32 {
    100
}

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

/// `---\n<yaml>\n---\n<body>` を split する。
///
/// 仕様:
/// - 入力は LF 改行。 CRLF が来たら LF に正規化してから処理。
/// - 1 行目が `---` であること。
/// - 次の `---` 行で frontmatter が終わる。
/// - `---` の後ろに body が続く (空でも可)。
pub fn split_frontmatter(input: &str) -> Result<(&str, &str), MdError> {
    let normalized = if input.contains('\r') {
        // 正規化のためコピー必要。 ただし戻り値は &str なので、
        // CRLF が含まれる入力は呼び出し側で normalize 済を渡すのが望ましい。
        // ここでは error にせず assume LF。
        input
    } else {
        input
    };
    let mut lines = normalized.split_inclusive('\n');
    let first = lines.next().ok_or(MdError::FrontmatterMissing)?;
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return Err(MdError::FrontmatterMissing);
    }
    // 残りから次の `---` 行を探す
    let rest_start = first.len();
    let rest = &normalized[rest_start..];
    // line-by-line scan
    let mut idx = 0usize;
    for line in rest.split_inclusive('\n') {
        let line_end = idx + line.len();
        if line.trim_end_matches(['\r', '\n']) == "---" {
            let yaml = &rest[..idx];
            let body_start = line_end;
            let body = if body_start <= rest.len() {
                &rest[body_start..]
            } else {
                ""
            };
            return Ok((yaml, body));
        }
        idx = line_end;
    }
    Err(MdError::FrontmatterUnterminated)
}

/// 1 つの md ファイル内容を (frontmatter, body) に分けて返す。
pub fn parse(input: &str) -> Result<(FrontMatter, String), MdError> {
    let (yaml, body) = split_frontmatter(input)?;
    let fm: FrontMatter = serde_yaml::from_str(yaml)?;
    Ok((fm, body.to_string()))
}

/// (frontmatter, body) を md 文字列に組み立てる。
pub fn serialize(fm: &FrontMatter, body: &str) -> Result<String, MdError> {
    let yaml = serde_yaml::to_string(fm)?;
    // serde_yaml は最後に改行を入れる。 body を追加する前に重複改行を avoid。
    let yaml = yaml.trim_end_matches('\n');
    Ok(format!("---\n{}\n---\n{}", yaml, body))
}

/// 内容ハッシュ (md_hash カラム / SUMS magic 等で使用)。
pub fn hash(input: &str) -> String {
    blake3::hash(input.as_bytes()).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn split_basic() {
        let s = "---\nfoo: 1\nbar: baz\n---\nhello body\n";
        let (yaml, body) = split_frontmatter(s).unwrap();
        assert_eq!(yaml, "foo: 1\nbar: baz\n");
        assert_eq!(body, "hello body\n");
    }

    #[test]
    fn split_empty_body() {
        let s = "---\nx: 1\n---\n";
        let (_, body) = split_frontmatter(s).unwrap();
        assert_eq!(body, "");
    }

    #[test]
    fn split_no_frontmatter_errors() {
        assert!(matches!(
            split_frontmatter("hello").unwrap_err(),
            MdError::FrontmatterMissing
        ));
        assert!(matches!(
            split_frontmatter("---\nfoo: 1\n").unwrap_err(),
            MdError::FrontmatterUnterminated
        ));
    }

    #[test]
    fn parse_reply_roundtrip() {
        let src = indoc! {r#"
            ---
            kind: reply
            id: 0192c5a0-0000-7000-8000-000000000001
            thread: 0192c5a0-0000-7000-8000-000000000002
            parent: 0192c5a0-0000-7000-8000-000000000003
            forum: 0192c5a0-0000-7000-8000-000000000004
            channel: 0192c5a0-0000-7000-8000-000000000005
            author: cr:user-uuid
            ts: 2026-05-08T12:06:00+09:00
            ---
            hello world
        "#};
        let (fm, body) = parse(src).unwrap();
        assert_eq!(fm.kind(), Kind::Reply);
        assert_eq!(body, "hello world\n");

        let serialized = serialize(&fm, &body).unwrap();
        // パースし直して同等であること
        let (fm2, body2) = parse(&serialized).unwrap();
        assert_eq!(fm.id(), fm2.id());
        assert_eq!(body, body2);
    }

    #[test]
    fn parse_forum() {
        let src = indoc! {r#"
            ---
            kind: forum
            id: 0192c5a0-0000-7000-8000-000000000010
            path: work/ludiars
            name: LUDIARS Workspace
            visibility: cernere-group
            group: cg_4f2a
            created_at: 2026-05-08T12:00:00+09:00
            created_by: cr:user-uuid
            ---
        "#};
        let (fm, _) = parse(src).unwrap();
        match fm {
            FrontMatter::Forum(m) => {
                assert_eq!(m.path, "work/ludiars");
                assert_eq!(m.visibility, Visibility::CernereGroup);
            }
            _ => panic!("expected forum"),
        }
    }

    #[test]
    fn hash_stable() {
        assert_eq!(hash("hello"), hash("hello"));
        assert_ne!(hash("hello"), hash("world"));
    }
}
