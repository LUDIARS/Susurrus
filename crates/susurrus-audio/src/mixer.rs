//! 受信側距離減衰 + パンミキサ。
//!
//! - 入力: 1 frame の mono PCM + 送信者の SpatialPosition + 自分の SpatialPosition
//! - 出力: ステレオ PCM (左右に減衰 + パン適用)
//!
//! 距離モデル / pan は v1.0 の最低限。 セクタ / 部屋 / inverse-square はあとで追加。

use susurrus_sdk::types::SpatialPosition;

/// 距離 → 0.0..=1.0 の単純な linear。
fn linear_atten(d: f32, min: f32, max: f32) -> f32 {
    if d <= min { 1.0 }
    else if d >= max { 0.0 }
    else { 1.0 - (d - min) / (max - min) }
}

/// 自分から相手への angle で pan を計算 (-1.0 = 左、 +1.0 = 右)。
/// y 軸を「上下」、 z 軸を「奥行き」、 x を左右と仮定 (Unity 風)。
/// 上下 (y) は無視 (現状)。
fn pan_from_offset(dx: f32, dz: f32) -> f32 {
    let mag = (dx * dx + dz * dz).sqrt().max(1e-6);
    (dx / mag).clamp(-1.0, 1.0)
}

/// PCM サンプル列に gain を適用しつつ stereo にミックスする。
///
/// `mix_buffer` は [L0, R0, L1, R1, ...] interleave。 同じ buffer に複数 source を加算する。
pub fn mix_into_stereo(
    mono_in: &[i16],
    listener: &SpatialPosition,
    speaker: &SpatialPosition,
    mix_buffer: &mut [i32], // i32 で加算してオーバーフロー回避
    min_d: f32,
    max_d: f32,
) {
    let dx = speaker.x - listener.x;
    let dy = speaker.y - listener.y;
    let dz = speaker.z - listener.z;
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    let gain = linear_atten(dist, min_d, max_d);
    if gain <= 0.0 {
        return;
    }
    let pan = pan_from_offset(dx, dz);
    // equal-power pan (-1 = full left, +1 = full right)
    let theta = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
    let l = theta.cos() * gain;
    let r = theta.sin() * gain;

    for (i, &s) in mono_in.iter().enumerate() {
        let li = i * 2;
        let ri = li + 1;
        if ri >= mix_buffer.len() { break; }
        mix_buffer[li] += (s as f32 * l) as i32;
        mix_buffer[ri] += (s as f32 * r) as i32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: f32, y: f32, z: f32) -> SpatialPosition {
        SpatialPosition { x, y, z, qx: 0.0, qy: 0.0, qz: 0.0, qw: 0.0 }
    }

    #[test]
    fn distant_speaker_silenced() {
        let mono = vec![10_000i16; 480];
        let mut mix = vec![0i32; 960];
        mix_into_stereo(
            &mono,
            &pos(0.0, 0.0, 0.0),    // listener
            &pos(100.0, 0.0, 0.0),  // 100m 離れた speaker
            &mut mix, 1.0, 50.0,
        );
        let total: i64 = mix.iter().map(|&x| x as i64).sum();
        assert_eq!(total, 0, "speaker beyond max should be silenced");
    }

    #[test]
    fn left_speaker_louder_on_left() {
        let mono = vec![10_000i16; 480];
        let mut mix = vec![0i32; 960];
        mix_into_stereo(
            &mono,
            &pos(0.0, 0.0, 0.0),
            &pos(-1.0, 0.0, 0.5),    // やや左
            &mut mix, 0.5, 10.0,
        );
        // L チャンネル合計 vs R チャンネル合計
        let l: i64 = mix.iter().step_by(2).map(|&x| x as i64).sum();
        let r: i64 = mix.iter().skip(1).step_by(2).map(|&x| x as i64).sum();
        assert!(l > r, "left source should be louder on left: l={l} r={r}");
    }
}
