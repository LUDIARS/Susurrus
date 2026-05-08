//! Rust 向け SDK API。 Tauri の HTTP server (port 17370 既定) を叩く。

use crate::types::{ReplyView, SpatialPosition};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("server: {status} - {body}")]
    Server { status: u16, body: String },
    #[error("decode: {0}")]
    Decode(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct Susurrus {
    pub endpoint: String,
    http: reqwest::Client,
}

impl Susurrus {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .expect("reqwest"),
        }
    }

    pub fn local_default() -> Self {
        Self::new("http://127.0.0.1:17370")
    }

    pub async fn ping(&self) -> Result<String, SdkError> {
        let resp = self.http.get(format!("{}/v1/ping", self.endpoint)).send().await?;
        Ok(resp.text().await?)
    }

    pub async fn list_replies(&self, thread_id: &str) -> Result<Vec<ReplyView>, SdkError> {
        let url = format!("{}/v1/threads/{thread_id}/replies", self.endpoint);
        let resp = self.http.get(url).send().await?;
        let status = resp.status();
        let bytes = resp.bytes().await?;
        if !status.is_success() {
            return Err(SdkError::Server {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&bytes).into_owned(),
            });
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub async fn send_reply(
        &self,
        thread_id: &str,
        author: &str,
        body: &str,
    ) -> Result<String, SdkError> {
        let url = format!("{}/v1/threads/{thread_id}/replies", self.endpoint);
        let resp = self
            .http
            .post(url)
            .json(&serde_json::json!({
                "author": author,
                "body": body,
            }))
            .send()
            .await?;
        let status = resp.status();
        let bytes = resp.bytes().await?;
        if !status.is_success() {
            return Err(SdkError::Server {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&bytes).into_owned(),
            });
        }
        let v: SendReplyResp = serde_json::from_slice(&bytes)?;
        Ok(v.id)
    }

    pub async fn send_typing(&self, thread_id: &str, user_uri: &str) -> Result<(), SdkError> {
        let url = format!("{}/v1/threads/{thread_id}/typing", self.endpoint);
        self.http
            .post(url)
            .json(&serde_json::json!({ "user": user_uri }))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn report_position(
        &self,
        user_uri: &str,
        forum_id: &str,
        pos: SpatialPosition,
    ) -> Result<(), SdkError> {
        let url = format!("{}/v1/spatial/position", self.endpoint);
        self.http
            .post(url)
            .json(&serde_json::json!({
                "user": user_uri,
                "forum_id": forum_id,
                "position": pos,
            }))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct SendReplyResp {
    id: String,
}
