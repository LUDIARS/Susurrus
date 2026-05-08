//! Cernere token verify。 起動時 1 回。

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CernereUser {
    /// "cr:<uuid>" 形式の URI
    pub user_uri: String,
}

pub async fn verify(_endpoint: &str, _token: &str) -> anyhow::Result<CernereUser> {
    // TODO: reqwest で endpoint に GET /me、 token を Bearer で渡す
    unimplemented!()
}
