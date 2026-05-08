//! Opus encode / decode wrapper。 audiopus 0.3 を使う。

use crate::FRAME_SAMPLES;
use audiopus::{coder::{Decoder, Encoder}, Application, Channels, MutSignals, SampleRate};

#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("audiopus: {0:?}")]
    Audiopus(String),
}

impl From<audiopus::Error> for CodecError {
    fn from(e: audiopus::Error) -> Self { CodecError::Audiopus(format!("{e:?}")) }
}

pub struct OpusEncoder {
    inner: Encoder,
}

impl OpusEncoder {
    /// 音声向け 48kHz mono encoder。 ビットレートは 24kbps の VoIP 想定。
    pub fn new_voice() -> Result<Self, CodecError> {
        let mut e = Encoder::new(
            SampleRate::Hz48000,
            Channels::Mono,
            Application::Voip,
        )?;
        // 24 kbps target
        e.set_bitrate(audiopus::Bitrate::BitsPerSecond(24_000))?;
        Ok(Self { inner: e })
    }

    /// 1 フレーム (mono PCM 960 sample) を encode → bytes。
    pub fn encode(&mut self, pcm: &[i16]) -> Result<Vec<u8>, CodecError> {
        if pcm.len() != FRAME_SAMPLES {
            return Err(CodecError::Audiopus(format!(
                "expected {} samples, got {}",
                FRAME_SAMPLES,
                pcm.len()
            )));
        }
        let mut out = vec![0u8; 4_000];
        let n = self.inner.encode(pcm, &mut out)?;
        out.truncate(n);
        Ok(out)
    }
}

pub struct OpusDecoder {
    inner: Decoder,
}

impl OpusDecoder {
    pub fn new_voice() -> Result<Self, CodecError> {
        Ok(Self {
            inner: Decoder::new(SampleRate::Hz48000, Channels::Mono)?,
        })
    }

    /// 1 frame の Opus bytes → mono PCM 960 sample。
    pub fn decode(&mut self, encoded: &[u8]) -> Result<Vec<i16>, CodecError> {
        let mut out = vec![0i16; FRAME_SAMPLES];
        let n = self.inner.decode(
            Some(encoded.try_into()?),
            MutSignals::try_from(out.as_mut_slice())?,
            false,
        )?;
        out.truncate(n);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_then_decode_silence() {
        let mut enc = OpusEncoder::new_voice().unwrap();
        let mut dec = OpusDecoder::new_voice().unwrap();
        let silence = vec![0i16; FRAME_SAMPLES];
        let bytes = enc.encode(&silence).unwrap();
        assert!(!bytes.is_empty());
        let pcm = dec.decode(&bytes).unwrap();
        assert_eq!(pcm.len(), FRAME_SAMPLES);
        // 無音入力 → 出力もほぼ無音
        let max_amp = pcm.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0);
        assert!(max_amp < 50, "decoded silence amplitude too high: {max_amp}");
    }
}
