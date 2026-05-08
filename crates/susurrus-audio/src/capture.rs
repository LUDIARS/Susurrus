//! OS マイク入力 (cpal) → Opus encoded frames を mpsc で吐き出す。
//!
//! Tauri context から起動する想定。 device の選択は cpal::default_host を使う。

use crate::codec::OpusEncoder;
use crate::{FRAME_SAMPLES, SAMPLE_RATE};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("cpal: {0}")]
    Cpal(String),
    #[error("codec: {0}")]
    Codec(#[from] crate::codec::CodecError),
    #[error("no input device")]
    NoDevice,
}

/// マイクから取った PCM をフレーム化して Opus encode し、 callback に渡す。
/// callback は audio thread で呼ばれるため軽量にすること (ロックや Tokio await は禁止)。
pub fn start_capture<F: Fn(Vec<u8>) + Send + Sync + 'static>(
    on_frame: F,
) -> Result<cpal::Stream, CaptureError> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or(CaptureError::NoDevice)?;
    let cfg = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(SAMPLE_RATE),
        buffer_size: cpal::BufferSize::Default,
    };
    let encoder = Arc::new(Mutex::new(OpusEncoder::new_voice()?));
    let mut accumulator: Vec<i16> = Vec::with_capacity(FRAME_SAMPLES * 2);

    let on_frame = Arc::new(on_frame);
    let encoder_for_cb = encoder.clone();
    let on_frame_for_cb = on_frame.clone();
    let stream = device
        .build_input_stream(
            &cfg,
            move |data: &[i16], _info: &cpal::InputCallbackInfo| {
                accumulator.extend_from_slice(data);
                while accumulator.len() >= FRAME_SAMPLES {
                    let frame: Vec<i16> = accumulator.drain(..FRAME_SAMPLES).collect();
                    // 同期 lock — encoder は audio callback 内で順次実行
                    let mut enc = match encoder_for_cb.try_lock() {
                        Ok(g) => g,
                        Err(_) => continue, // skip frame if locked
                    };
                    if let Ok(bytes) = enc.encode(&frame) {
                        on_frame_for_cb(bytes);
                    }
                }
            },
            |err| tracing::warn!("audio capture stream error: {err}"),
            None,
        )
        .map_err(|e| CaptureError::Cpal(format!("{e}")))?;

    stream.play().map_err(|e| CaptureError::Cpal(format!("{e}")))?;
    Ok(stream)
}
