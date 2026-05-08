use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplyView {
    pub id: String,
    pub author: String,
    pub ts: String,
    pub body: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SpatialPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// 任意の orientation (四元数 w, x, y, z)。 不要なら全部 0
    #[serde(default)]
    pub qx: f32,
    #[serde(default)]
    pub qy: f32,
    #[serde(default)]
    pub qz: f32,
    #[serde(default)]
    pub qw: f32,
}
