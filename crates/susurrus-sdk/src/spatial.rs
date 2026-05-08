//! Spatial Chat (v1.0+) 用ヘルパ。
//!
//! ホストアプリは player の位置を [`SpatialPosition`] で SDK に渡す。
//! 距離計算 / mute 判定は受信側 SDK が行う設計 (server なしの spatial)。

pub use crate::types::SpatialPosition;

/// 2 点間の距離 (3D)。
pub fn distance(a: &SpatialPosition, b: &SpatialPosition) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// 距離減衰 (linear、 max を超えたら 0)。
pub fn linear_attenuation(d: f32, min: f32, max: f32) -> f32 {
    if d <= min { 1.0 }
    else if d >= max { 0.0 }
    else { 1.0 - (d - min) / (max - min) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SpatialPosition;

    fn p(x: f32, y: f32, z: f32) -> SpatialPosition {
        SpatialPosition { x, y, z, qx: 0.0, qy: 0.0, qz: 0.0, qw: 0.0 }
    }

    #[test]
    fn distance_basic() {
        assert_eq!(distance(&p(0.0, 0.0, 0.0), &p(3.0, 4.0, 0.0)), 5.0);
    }

    #[test]
    fn attenuation_curve() {
        assert_eq!(linear_attenuation(0.0, 1.0, 10.0), 1.0);
        assert_eq!(linear_attenuation(10.0, 1.0, 10.0), 0.0);
        assert!((linear_attenuation(5.5, 1.0, 10.0) - 0.5).abs() < 0.001);
    }
}
