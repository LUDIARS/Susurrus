//! Opus decode + spatial mixer → cpal 出力。

use crate::codec::OpusDecoder;
use crate::mixer::mix_into_stereo;
use crate::{FRAME_SAMPLES, SAMPLE_RATE};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use susurrus_sdk::types::SpatialPosition;

#[derive(Debug, thiserror::Error)]
pub enum PlaybackError {
    #[error("cpal: {0}")]
    Cpal(String),
    #[error("codec: {0}")]
    Codec(#[from] crate::codec::CodecError),
    #[error("no output device")]
    NoDevice,
}

/// peer ごとの位置 + 直近 frame バッファを管理。
#[derive(Default)]
pub struct PlaybackState {
    pub listener: SpatialPosition,
    pub min_d: f32,
    pub max_d: f32,
    /// peer_id → SpatialPosition
    pub peers: HashMap<String, SpatialPosition>,
    /// peer_id → 最新 PCM (再生待ち)
    pub pending: HashMap<String, Vec<i16>>,
}

/// 受信した Opus frame を decode → state.pending に push。
pub fn ingest_opus(
    state: &mut PlaybackState,
    decoder: &mut OpusDecoder,
    peer_id: &str,
    encoded: &[u8],
) -> Result<(), PlaybackError> {
    let pcm = decoder.decode(encoded)?;
    state
        .pending
        .entry(peer_id.to_string())
        .or_default()
        .extend(pcm);
    Ok(())
}

pub fn start_playback(state: Arc<Mutex<PlaybackState>>) -> Result<cpal::Stream, PlaybackError> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(PlaybackError::NoDevice)?;
    let cfg = cpal::StreamConfig {
        channels: 2,
        sample_rate: cpal::SampleRate(SAMPLE_RATE),
        buffer_size: cpal::BufferSize::Default,
    };

    let stream = device
        .build_output_stream(
            &cfg,
            move |out: &mut [i16], _info: &cpal::OutputCallbackInfo| {
                let mut g = match state.try_lock() {
                    Ok(g) => g,
                    Err(_) => {
                        out.fill(0);
                        return;
                    }
                };
                let needed_stereo = out.len();
                let needed_mono = needed_stereo / 2;
                let mut mix = vec![0i32; needed_stereo];

                let listener = g.listener.clone();
                let min_d = g.min_d;
                let max_d = g.max_d;
                let peers_clone: Vec<(String, SpatialPosition)> = g
                    .peers
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                for (pid, pos) in peers_clone {
                    let buf = g.pending.entry(pid.clone()).or_default();
                    if buf.len() < needed_mono {
                        // 不足分は 0 padding
                        let pad = needed_mono - buf.len();
                        buf.extend(std::iter::repeat(0i16).take(pad));
                    }
                    let take: Vec<i16> = buf.drain(..needed_mono).collect();
                    mix_into_stereo(&take, &listener, &pos, &mut mix, min_d, max_d);
                }

                for (i, sample) in mix.iter().enumerate() {
                    let clamped = (*sample).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                    out[i] = clamped;
                }
                let _ = (needed_stereo, FRAME_SAMPLES);
            },
            |err| tracing::warn!("audio playback stream error: {err}"),
            None,
        )
        .map_err(|e| PlaybackError::Cpal(format!("{e}")))?;

    stream
        .play()
        .map_err(|e| PlaybackError::Cpal(format!("{e}")))?;
    Ok(stream)
}

// SpatialPosition は susurrus-sdk で Clone + Copy + Default 派生済み
