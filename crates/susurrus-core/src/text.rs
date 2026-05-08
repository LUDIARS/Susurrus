//! Markdown 本文 → FTS 用プレーンテキスト抽出。

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

/// Markdown のテキスト要素のみを連結した plain string を返す (FTS index 用)。
/// 見出し / 段落 / リスト / blockquote の境目には半角スペースを入れる。
pub fn to_plain(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    let mut last_was_text = false;
    for ev in Parser::new(md) {
        match ev {
            Event::Text(t) | Event::Code(t) => {
                if last_was_text {
                    out.push(' ');
                }
                out.push_str(&t);
                last_was_text = true;
            }
            Event::SoftBreak | Event::HardBreak | Event::Rule => {
                out.push(' ');
                last_was_text = false;
            }
            Event::Start(Tag::Paragraph)
            | Event::End(TagEnd::Paragraph)
            | Event::Start(Tag::Heading { .. })
            | Event::End(TagEnd::Heading(_))
            | Event::Start(Tag::List(_))
            | Event::End(TagEnd::List(_))
            | Event::Start(Tag::Item)
            | Event::End(TagEnd::Item)
            | Event::Start(Tag::BlockQuote(_))
            | Event::End(TagEnd::BlockQuote(_))
            | Event::Start(Tag::CodeBlock(_))
            | Event::End(TagEnd::CodeBlock) => {
                out.push(' ');
                last_was_text = false;
            }
            _ => {}
        }
    }
    // 連続する空白を 1 つに圧縮
    let mut compact = String::with_capacity(out.len());
    let mut prev_ws = false;
    for ch in out.chars() {
        if ch.is_whitespace() {
            if !prev_ws {
                compact.push(' ');
            }
            prev_ws = true;
        } else {
            compact.push(ch);
            prev_ws = false;
        }
    }
    compact.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_markdown() {
        let md = "# Title\n\nThis is a **bold** paragraph with [link](http://x).\n\n- item one\n- item two\n";
        let s = to_plain(md);
        assert!(s.contains("Title"));
        assert!(s.contains("paragraph"));
        assert!(s.contains("item one"));
        assert!(s.contains("item two"));
        assert!(!s.contains("**"));
        assert!(!s.contains("http://x"));
    }
}
