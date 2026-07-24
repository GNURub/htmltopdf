use crate::{dom::Document, RenderError};
use std::collections::BTreeMap;

const VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

pub fn parse_document(html: &str) -> Result<Document, RenderError> {
    if html.trim().is_empty() {
        return Err(RenderError::InvalidInput(
            "HTML document is empty".to_string(),
        ));
    }

    let mut document = Document::new();
    let mut stack = vec![document.root()];
    let mut cursor = 0;

    while cursor < html.len() {
        let Some(open_rel) = html[cursor..].find('<') else {
            push_text(&mut document, *stack.last().unwrap_or(&0), &html[cursor..]);
            break;
        };
        let open = cursor + open_rel;
        push_text(
            &mut document,
            *stack.last().unwrap_or(&0),
            &html[cursor..open],
        );

        if html[open..].starts_with("<!--") {
            let comment_start = open + "<!--".len();
            let Some(end_rel) = html[comment_start..].find("-->") else {
                break;
            };
            cursor = comment_start + end_rel + "-->".len();
            continue;
        }

        let Some(close_rel) = html[open..].find('>') else {
            push_text(&mut document, *stack.last().unwrap_or(&0), &html[open..]);
            break;
        };
        let close = open + close_rel;
        let token = html[open + 1..close].trim();
        cursor = close + 1;

        if token.is_empty() || token.starts_with('!') || token.starts_with('?') {
            continue;
        }

        if let Some(end_tag) = token.strip_prefix('/') {
            let end_tag = end_tag
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            while stack.len() > 1 {
                let popped = stack.pop().unwrap_or(0);
                let is_match = match document.node(popped) {
                    Some(crate::dom::Node::Element(element)) => element.tag == end_tag,
                    _ => false,
                };
                if is_match {
                    break;
                }
            }
            continue;
        }

        let self_closing = token.ends_with('/');
        let token = token.trim_end_matches('/').trim();
        let (tag, attrs) = parse_start_tag(token);
        if tag.is_empty() {
            continue;
        }
        let parent = *stack.last().unwrap_or(&document.root());
        let id = document.push_element(parent, tag.clone(), attrs);

        if tag == "script" || tag == "style" {
            let end_marker = format!("</{tag}>");
            if let Some(end_rel) = html[cursor..].to_ascii_lowercase().find(&end_marker) {
                let end = cursor + end_rel;
                document.push_text(id, html[cursor..end].to_string());
                cursor = end + end_marker.len();
            }
            continue;
        }

        if !self_closing && !VOID_TAGS.contains(&tag.as_str()) {
            stack.push(id);
        }
    }

    Ok(document)
}

fn push_text(document: &mut Document, parent: usize, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    document.push_text(parent, decode_entities(text));
}

fn parse_start_tag(token: &str) -> (String, BTreeMap<String, String>) {
    let mut chars = token.chars().peekable();
    let mut tag = String::new();
    while let Some(ch) = chars.peek().copied() {
        if ch.is_whitespace() {
            break;
        }
        tag.push(ch.to_ascii_lowercase());
        let _ = chars.next();
    }

    let rest: String = chars.collect();
    (tag, parse_attrs(&rest))
}

fn parse_attrs(input: &str) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let key_start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'=' {
            i += 1;
        }
        let key = input[key_start..i].trim().to_ascii_lowercase();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        let mut value = String::new();
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let quote = bytes[i];
                i += 1;
                let value_start = i;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                value = decode_entities(&input[value_start..i]);
                if i < bytes.len() {
                    i += 1;
                }
            } else {
                let value_start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                value = decode_entities(&input[value_start..i]);
            }
        }

        if !key.is_empty() {
            attrs.insert(key, value);
        }
    }

    attrs
}

fn decode_entities(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;

    while let Some(relative_ampersand) = input[cursor..].find('&') {
        let ampersand = cursor + relative_ampersand;
        output.push_str(&input[cursor..ampersand]);

        let entity_start = ampersand + 1;
        let Some(relative_semicolon) = input[entity_start..].find(';') else {
            output.push('&');
            cursor = entity_start;
            continue;
        };
        let semicolon = entity_start + relative_semicolon;
        let entity = &input[entity_start..semicolon];
        if let Some(character) = decode_entity(entity) {
            output.push(character);
        } else {
            output.push_str(&input[ampersand..=semicolon]);
        }
        cursor = semicolon + 1;
    }

    output.push_str(&input[cursor..]);
    output
}

fn decode_entity(entity: &str) -> Option<char> {
    if let Some(value) = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
    {
        return u32::from_str_radix(value, 16)
            .ok()
            .and_then(decode_codepoint);
    }
    if let Some(value) = entity.strip_prefix('#') {
        return value.parse::<u32>().ok().and_then(decode_codepoint);
    }

    let name = entity.to_ascii_lowercase();
    let character = match name.as_str() {
        "nbsp" => '\u{00a0}',
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "copy" => '\u{00a9}',
        "reg" => '\u{00ae}',
        "trade" => '\u{2122}',
        "euro" => '\u{20ac}',
        "pound" => '\u{00a3}',
        "yen" => '\u{00a5}',
        "cent" => '\u{00a2}',
        "curren" => '\u{00a4}',
        "deg" => '\u{00b0}',
        "plusmn" => '\u{00b1}',
        "micro" => '\u{00b5}',
        "para" => '\u{00b6}',
        "middot" => '\u{00b7}',
        "times" => '\u{00d7}',
        "divide" => '\u{00f7}',
        "ndash" => '\u{2013}',
        "mdash" => '\u{2014}',
        "lsquo" => '\u{2018}',
        "rsquo" => '\u{2019}',
        "ldquo" => '\u{201c}',
        "rdquo" => '\u{201d}',
        "bull" => '\u{2022}',
        "hellip" => '\u{2026}',
        "laquo" => '\u{00ab}',
        "raquo" => '\u{00bb}',
        "sect" => '\u{00a7}',
        "frac14" => '\u{00bc}',
        "frac12" => '\u{00bd}',
        "frac34" => '\u{00be}',
        _ => return None,
    };
    Some(character)
}

fn decode_codepoint(value: u32) -> Option<char> {
    match value {
        0 | 0xd800..=0xdfff | 0x110000.. => Some('\u{fffd}'),
        value => char::from_u32(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_comments_even_when_they_contain_tag_like_text() {
        let document =
            parse_document("<p>Before<!-- ignored > <span>still ignored</span> -->After</p>")
                .expect("valid html");
        let paragraph = document.query_selector("p").expect("paragraph exists");

        assert_eq!(document.text_content(paragraph), "Before After");
        assert!(document.query_selector("span").is_none());
    }

    #[test]
    fn decodes_named_and_numeric_entities_in_text() {
        assert_eq!(
            decode_entities("&lt;invoice&gt; &amp; &#x20AC; &#128640; &unknown;"),
            "<invoice> & € 🚀 &unknown;"
        );
    }

    #[test]
    fn decodes_entities_in_attributes() {
        let document = parse_document(r#"<div title="A &quot;quote&quot; &amp; more"></div>"#)
            .expect("valid html");
        let node = document.query_selector("div").expect("div exists");

        let Some(crate::dom::Node::Element(element)) = document.node(node) else {
            panic!("expected element node");
        };
        assert_eq!(element.attr("title"), Some("A \"quote\" & more"));
    }
}
