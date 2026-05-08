//! Susurrus 音声処理 (v1.0 Spatial Chat 用)。
//!
//! - [`codec`] = Opus encode / decode (audiopus crate)
//! - [`capture`] = OS マイク入力 (cpal) → Opus encoded frames
//! - [`playback`] = Opus decode + spatial mixer (距離減衰) → OS 出力 (cpal)
//! - [`mixer`] = 距離減衰 + ステレオ pan (受信側で完結)
//!
//! フレーム = 20ms 単位 (48kHz mono = 960 samples)。 1 frame ≈ 80-160 bytes encoded。

pub mod capture;
pub mod codec;
pub mod mixer;
pub mod playback;

/// 標準フレームレート (sample rate)。 ステレオ playback では同 rate でアップミックス。
pub const SAMPLE_RATE: u32 = 48_000;
/// 1 フレームの ms 数。 Opus が 2.5/5/10/20/40/60 ms を許容、 Susurrus は 20 で固定。
pub const FRAME_MS: u32 = 20;
/// 1 フレームのサンプル数 (mono)。
pub const FRAME_SAMPLES: usize = (SAMPLE_RATE as usize * FRAME_MS as usize) / 1000;
