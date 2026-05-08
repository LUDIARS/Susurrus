//! Memoria 連携 (opt-out 可、 既定 on)。
//!
//! 接続先 = Memoria のローカル HTTP API。 v0.0 では雛形 endpoint のみ用意し、
//! 実際の Memoria 側 endpoint 名 (例: `/api/bookmarks`) が確定したら差し替える。
//!
//! 機能:
//! - `save_bookmark` — メッセージ → ブクマ
//! - `request_dig` — URL → Memoria Dig (要約) を依頼
//! - `delegate_task` — メンションで Memoria にタスク委託 (将来)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum MemoriaError {
    #[error("disabled: Memoria 連携は無効化されています")]
    Disabled,
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("response: {status} - {body}")]
    Response { status: u16, body: String },
}

#[derive(Debug, Clone)]
pub struct MemoriaClient {
    pub endpoint: String,
    pub token: Option<String>,
    pub enabled: bool,
    http: reqwest::Client,
}

impl MemoriaClient {
    pub fn new(endpoint: impl Into<String>, token: Option<String>, enabled: bool) -> Self {
        Self {
            endpoint: endpoint.into(),
            token,
            enabled,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("reqwest client build"),
        }
    }

    fn require_enabled(&self) -> Result<(), MemoriaError> {
        if !self.enabled { Err(MemoriaError::Disabled) } else { Ok(()) }
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(t) = &self.token {
            req.bearer_auth(t)
        } else {
            req
        }
    }

    pub async fn save_bookmark(&self, b: &SaveBookmark) -> Result<SavedBookmark, MemoriaError> {
        self.require_enabled()?;
        let url = format!("{}/api/bookmarks", self.endpoint.trim_end_matches('/'));
        let resp = self.auth(self.http.post(&url)).json(b).send().await?;
        let status = resp.status();
        let bytes = resp.bytes().await?;
        if !status.is_success() {
            return Err(MemoriaError::Response {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&bytes).into_owned(),
            });
        }
        let saved: SavedBookmark = serde_json::from_slice(&bytes)
            .map_err(|e| MemoriaError::Response {
                status: status.as_u16(),
                body: format!("decode: {e}"),
            })?;
        Ok(saved)
    }

    pub async fn request_dig(&self, url: &str) -> Result<DigResult, MemoriaError> {
        self.require_enabled()?;
        let endpoint = format!("{}/api/dig", self.endpoint.trim_end_matches('/'));
        let resp = self
            .auth(self.http.post(&endpoint))
            .json(&serde_json::json!({ "url": url }))
            .send()
            .await?;
        let status = resp.status();
        let bytes = resp.bytes().await?;
        if !status.is_success() {
            return Err(MemoriaError::Response {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&bytes).into_owned(),
            });
        }
        let dig: DigResult = serde_json::from_slice(&bytes)
            .map_err(|e| MemoriaError::Response {
                status: status.as_u16(),
                body: format!("decode: {e}"),
            })?;
        Ok(dig)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveBookmark {
    pub url: Option<String>,
    pub title: String,
    pub body: String,
    pub source: String, // 例: "susurrus:reply:<reply_id>"
    pub tags: Vec<String>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedBookmark {
    pub id: String,
    pub url: Option<String>,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigResult {
    pub url: String,
    pub summary: String,
    pub tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn disabled_returns_disabled_error() {
        let c = MemoriaClient::new("http://127.0.0.1:5180", None, false);
        let r = c.save_bookmark(&SaveBookmark {
            url: None, title: "t".into(), body: "b".into(),
            source: "test".into(), tags: vec![], created_at: None,
        }).await;
        assert!(matches!(r, Err(MemoriaError::Disabled)));
    }
}
