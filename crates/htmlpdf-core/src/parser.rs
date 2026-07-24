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
    input
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}
