//! Stream ペイロード型。 CBOR (ciborium) でエンコード。
//!
//! 各型は magic と 1:1。 frame size 制限は受信側で定める (デフォルト 64 KiB)。

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SusMsg {
    pub msg_id: Uuid,
    pub thread_id: Uuid,
    pub forum_id: Uuid,
    /// blake3(md content)
    #[serde(with = "serde_bytes")]
    pub md_hash: Vec<u8>,
    /// blake3 of Synergos content (= CID)
    #[serde(with = "serde_bytes")]
    pub md_cid: Vec<u8>,
    /// 256 byte 程度に切った先頭 (UI 即時表示用)
    pub preview: String,
    pub ts_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SusTyping {
    pub thread_id: Uuid,
    pub user_uri: String,
    pub until_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SusRead {
    pub thread_id: Uuid,
    pub user_uri: String,
    pub last_read_reply_id: Option<Uuid>,
    pub last_read_ts_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SusReact {
    pub reply_id: Uuid,
    pub thread_id: Uuid,
    pub emoji: String,
    pub user_uri: String,
    /// false = 取り消し
    pub add: bool,
    pub ts_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SusPing {
    pub nonce: u64,
    pub ts_ms: i64,
}

/// CBOR バイト列にエンコード。
pub fn encode<T: Serialize>(v: &T) -> Result<Vec<u8>, ciborium::ser::Error<std::io::Error>> {
    let mut buf = Vec::new();
    ciborium::into_writer(v, &mut buf)?;
    Ok(buf)
}

/// CBOR バイト列からデコード。
pub fn decode<T: for<'de> Deserialize<'de>>(
    bytes: &[u8],
) -> Result<T, ciborium::de::Error<std::io::Error>> {
    ciborium::from_reader(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn roundtrip_susmsg() {
        let m = SusMsg {
            msg_id: Uuid::now_v7(),
            thread_id: Uuid::now_v7(),
            forum_id: Uuid::now_v7(),
            md_hash: vec![0xab; 32],
            md_cid: vec![0xcd; 32],
            preview: "hello world".into(),
            ts_ms: Utc::now().timestamp_millis(),
        };
        let bytes = encode(&m).unwrap();
        let back: SusMsg = decode(&bytes).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn roundtrip_typing() {
        let t = SusTyping {
            thread_id: Uuid::now_v7(),
            user_uri: "cr:user".into(),
            until_ms: 1_700_000_000_000,
        };
        let b = encode(&t).unwrap();
        let back: SusTyping = decode(&b).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn roundtrip_react() {
        let r = SusReact {
            reply_id: Uuid::now_v7(),
            thread_id: Uuid::now_v7(),
            emoji: "👍".into(),
            user_uri: "cr:user".into(),
            add: true,
            ts_ms: 1_700_000_000_000,
        };
        let b = encode(&r).unwrap();
        let back: SusReact = decode(&b).unwrap();
        assert_eq!(r, back);
    }
}
