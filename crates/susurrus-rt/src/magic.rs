//! Stream magic 4 byte の判定 + I/O ヘルパ。

use std::io;

/// すべての magic は ASCII 4 文字で、 `SU` プレフィックス + 2 文字 + 1 桁 version。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Magic {
    /// message commit notification
    Msg,
    /// typing
    Typing,
    /// read cursor update
    Read,
    /// reaction add/remove
    React,
    /// presence ping (RTT)
    Ping,
    /// spatial position (v1.0+)
    Spatial,
}

impl Magic {
    pub const fn bytes(self) -> [u8; 4] {
        match self {
            Self::Msg     => *b"SUM1",
            Self::Typing  => *b"SUT1",
            Self::Read    => *b"SUR1",
            Self::React   => *b"SUX1",
            Self::Ping    => *b"SUP1",
            Self::Spatial => *b"SUS1",
        }
    }

    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        let arr: [u8; 4] = b.get(..4)?.try_into().ok()?;
        match &arr {
            b"SUM1" => Some(Self::Msg),
            b"SUT1" => Some(Self::Typing),
            b"SUR1" => Some(Self::Read),
            b"SUX1" => Some(Self::React),
            b"SUP1" => Some(Self::Ping),
            b"SUS1" => Some(Self::Spatial),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Msg => "SUM1",
            Self::Typing => "SUT1",
            Self::Read => "SUR1",
            Self::React => "SUX1",
            Self::Ping => "SUP1",
            Self::Spatial => "SUS1",
        }
    }
}

/// magic 4 byte を頭に書く。 stream 単位で 1 度だけ呼ぶ。
pub fn write_magic<W: io::Write>(w: &mut W, m: Magic) -> io::Result<()> {
    w.write_all(&m.bytes())
}

/// 受信側で magic を読み出す。
pub fn read_magic<R: io::Read>(r: &mut R) -> io::Result<Magic> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Magic::from_bytes(&buf).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "unknown susurrus stream magic")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for m in [Magic::Msg, Magic::Typing, Magic::Read, Magic::React, Magic::Ping, Magic::Spatial] {
            let b = m.bytes();
            assert_eq!(Magic::from_bytes(&b), Some(m));
        }
    }

    #[test]
    fn unknown_returns_none() {
        assert_eq!(Magic::from_bytes(b"HLO1"), None);
        assert_eq!(Magic::from_bytes(b"SU"), None); // 短すぎ
    }

    #[test]
    fn write_then_read() {
        let mut buf: Vec<u8> = Vec::new();
        write_magic(&mut buf, Magic::Typing).unwrap();
        let mut cur = std::io::Cursor::new(&buf);
        assert_eq!(read_magic(&mut cur).unwrap(), Magic::Typing);
    }
}
