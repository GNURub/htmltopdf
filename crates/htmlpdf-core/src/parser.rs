use crate::{dom::Document, RenderError};
use std::collections::BTreeMap;

mod entities;

const VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParsingNamespace {
    Html,
    Svg,
    MathMl,
}

fn child_namespace(
    namespace: ParsingNamespace,
    parent: Option<&crate::dom::Node>,
    tag: &str,
) -> ParsingNamespace {
    let mut html_context = namespace == ParsingNamespace::Html;
    if let Some(crate::dom::Node::Element(element)) = parent {
        html_context |= match namespace {
            ParsingNamespace::Svg => {
                matches!(element.tag.as_str(), "foreignobject" | "desc" | "title")
            }
            ParsingNamespace::MathMl => {
                (matches!(element.tag.as_str(), "mi" | "mo" | "mn" | "ms" | "mtext")
                    && !matches!(tag, "mglyph" | "malignmark"))
                    || (element.tag == "annotation-xml"
                        && element.attr("encoding").is_some_and(|encoding| {
                            encoding.eq_ignore_ascii_case("text/html")
                                || encoding.eq_ignore_ascii_case("application/xhtml+xml")
                        }))
            }
            ParsingNamespace::Html => true,
        };
        if namespace == ParsingNamespace::MathMl && element.tag == "annotation-xml" && tag == "svg"
        {
            return ParsingNamespace::Svg;
        }
    }
    if html_context {
        match tag {
            "svg" => ParsingNamespace::Svg,
            "math" => ParsingNamespace::MathMl,
            _ => ParsingNamespace::Html,
        }
    } else {
        namespace
    }
}

pub fn parse_document(html: &str) -> Result<Document, RenderError> {
    if html.trim().is_empty() {
        return Err(RenderError::InvalidInput(
            "HTML document is empty".to_string(),
        ));
    }

    // HTML input preprocessing normalizes CRLF and bare CR before tokenization.
    // Avoid allocating a second copy for the common LF-only case.
    let normalized;
    let html = if html.contains('\r') {
        normalized = html.replace("\r\n", "\n").replace('\r', "\n");
        normalized.as_str()
    } else {
        html
    };
    let mut document = Document::new();
    let mut stack = vec![document.root()];
    let mut namespaces = BTreeMap::new();
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

        let tag_start = html.as_bytes().get(open + 1).copied();
        let valid_start = tag_start
            .is_some_and(|byte| byte.is_ascii_alphabetic() || matches!(byte, b'!' | b'?'))
            || (tag_start == Some(b'/')
                && html
                    .as_bytes()
                    .get(open + 2)
                    .is_some_and(u8::is_ascii_alphabetic));
        if !valid_start {
            document.push_text(*stack.last().unwrap_or(&0), "<");
            cursor = open + 1;
            continue;
        }
        let Some((close, self_closing)) = find_tag_end(html, open + 1) else {
            break;
        };
        let token = html[open + 1..close].trim_matches(is_html_space);
        cursor = close + 1;

        if token.is_empty() || token.starts_with('!') || token.starts_with('?') {
            continue;
        }

        if let Some(end_tag) = token.strip_prefix('/') {
            let end_tag = end_tag
                .split(|ch| is_html_space(ch) || ch == '/')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if let Some(index) = stack.iter().rposition(|id| {
                matches!(document.node(*id), Some(crate::dom::Node::Element(element)) if element.tag == end_tag)
            }) {
                stack.truncate(index.max(1));
            }
            continue;
        }

        let token = if self_closing {
            token
                .strip_suffix('/')
                .unwrap_or(token)
                .trim_end_matches(is_html_space)
        } else {
            token
        };
        let (tag, attrs) = parse_start_tag(token);
        if tag.is_empty() {
            continue;
        }
        let parent = *stack.last().unwrap_or(&document.root());
        let namespace = child_namespace(
            namespaces
                .get(&parent)
                .copied()
                .unwrap_or(ParsingNamespace::Html),
            document.node(parent),
            &tag,
        );
        let id = document.push_element(parent, tag.clone(), attrs);
        if namespace != ParsingNamespace::Html {
            namespaces.insert(id, namespace);
        }

        if namespace == ParsingNamespace::Html
            && matches!(tag.as_str(), "script" | "style" | "textarea" | "title")
        {
            let (end, next) =
                find_raw_text_end(html, cursor, &tag).unwrap_or((html.len(), html.len()));
            let content = &html[cursor..end];
            let mut text = if tag == "textarea" || tag == "title" {
                decode_entities(content)
            } else {
                content.to_string()
            };
            if tag == "textarea" && text.starts_with('\n') {
                text.remove(0);
            }
            document.push_text(id, text);
            cursor = next;
            continue;
        }

        let closes = if namespace == ParsingNamespace::Html {
            VOID_TAGS.contains(&tag.as_str())
        } else {
            self_closing
        };
        if !closes {
            stack.push(id);
        }
    }

    Ok(document)
}

fn is_html_space(ch: char) -> bool {
    matches!(ch, '\t' | '\n' | '\u{000c}' | '\r' | ' ')
}

// Only a quote at the beginning of an attribute value opens a quoted
// value. In particular, '>' inside that value is not a tag terminator.
fn find_tag_end(html: &str, start: usize) -> Option<(usize, bool)> {
    enum State {
        Tag,
        BeforeValue,
        Quoted(u8),
        Unquoted,
    }
    let mut state = State::Tag;
    let mut self_closing = false;
    for (offset, byte) in html.as_bytes()[start..].iter().copied().enumerate() {
        match state {
            State::Quoted(quote) => {
                if byte == quote {
                    state = State::Tag;
                }
            }
            State::BeforeValue => {
                if byte == b'>' {
                    return Some((start + offset, false));
                }
                if byte == b'\'' || byte == b'"' {
                    state = State::Quoted(byte);
                } else if !is_html_space(byte as char) {
                    state = State::Unquoted;
                }
            }
            State::Unquoted => {
                if byte == b'>' {
                    return Some((start + offset, false));
                }
                if is_html_space(byte as char) {
                    state = State::Tag;
                }
            }
            State::Tag => {
                if byte == b'>' {
                    return Some((start + offset, self_closing));
                }
                self_closing = byte == b'/';
                if byte == b'=' {
                    state = State::BeforeValue;
                }
            }
        }
    }
    None
}

fn find_raw_text_end(html: &str, start: usize, tag: &str) -> Option<(usize, usize)> {
    let marker = format!("</{tag}");
    let bytes = html.as_bytes();
    let mut cursor = start;
    while let Some(relative) = html[cursor..].find('<') {
        let open = cursor + relative;
        let name_end = open + marker.len();
        if bytes
            .get(open..name_end)
            .is_some_and(|name| name.eq_ignore_ascii_case(marker.as_bytes()))
            && bytes
                .get(name_end)
                .is_some_and(|byte| is_html_space(*byte as char) || matches!(byte, b'>' | b'/'))
        {
            if let Some((close, _)) = find_tag_end(html, name_end) {
                return Some((open, close + 1));
            }
        }
        cursor = open + 1;
    }
    None
}

fn push_text(document: &mut Document, parent: usize, text: &str) {
    if text.is_empty() {
        return;
    }
    document.push_text(parent, decode_entities(text));
}

fn parse_start_tag(token: &str) -> (String, BTreeMap<String, String>) {
    let end = token
        .find(|ch| is_html_space(ch) || ch == '/')
        .unwrap_or(token.len());
    (
        token[..end].to_ascii_lowercase(),
        parse_attrs(&token[end..]),
    )
}

fn parse_attrs(input: &str) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        while i < bytes.len() && (is_html_space(bytes[i] as char) || bytes[i] == b'/') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let key_start = i;
        while i < bytes.len()
            && !is_html_space(bytes[i] as char)
            && !matches!(bytes[i], b'=' | b'/')
        {
            i += 1;
        }
        let key = input[key_start..i].to_ascii_lowercase();
        while i < bytes.len() && is_html_space(bytes[i] as char) {
            i += 1;
        }

        let mut value = String::new();
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && is_html_space(bytes[i] as char) {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let quote = bytes[i];
                i += 1;
                let value_start = i;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                value = decode_entities_in_context(&input[value_start..i], true);
                if i < bytes.len() {
                    i += 1;
                }
            } else {
                let value_start = i;
                while i < bytes.len() && !is_html_space(bytes[i] as char) {
                    i += 1;
                }
                value = decode_entities_in_context(&input[value_start..i], true);
            }
        }

        if !key.is_empty() {
            attrs.entry(key).or_insert(value);
        }
    }

    attrs
}

fn decode_entities(input: &str) -> String {
    decode_entities_in_context(input, false)
}

fn decode_entities_in_context(input: &str, attribute: bool) -> String {
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    while let Some(relative) = input[cursor..].find('&') {
        let amp = cursor + relative;
        output.push_str(&input[cursor..amp]);
        let start = amp + 1;
        let tail = &input[start..];
        if let Some((consumed, character)) = numeric_reference(tail) {
            output.push(character);
            cursor = start + consumed;
            continue;
        }
        // Names are ASCII and at most 32 bytes. Never scan arbitrarily far
        // forward looking for a semicolon after an unknown ampersand.
        let length = tail
            .bytes()
            .take(32)
            .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b';')
            .count();
        let mut matched = None;
        for end in (1..=length).rev() {
            let name = &tail[..end];
            if let Ok(index) = entities::NAMED.binary_search_by_key(&name, |entry| entry.0) {
                let ambiguous = attribute
                    && !name.ends_with(';')
                    && tail
                        .as_bytes()
                        .get(end)
                        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'=');
                if !ambiguous {
                    matched = Some((end, entities::NAMED[index].1));
                }
                break;
            }
        }
        if let Some((consumed, text)) = matched {
            output.push_str(text);
            cursor = start + consumed;
        } else {
            output.push('&');
            cursor = start;
        }
    }
    output.push_str(&input[cursor..]);
    output
}

fn numeric_reference(input: &str) -> Option<(usize, char)> {
    let bytes = input.as_bytes();
    if bytes.first() != Some(&b'#') {
        return None;
    }
    let mut cursor = 1;
    let radix = if matches!(bytes.get(cursor), Some(b'x' | b'X')) {
        cursor += 1;
        16
    } else {
        10
    };
    let digit_start = cursor;
    let mut value = 0u32;
    while let Some(digit) = bytes
        .get(cursor)
        .and_then(|byte| (*byte as char).to_digit(radix))
    {
        value = value.saturating_mul(radix).saturating_add(digit);
        cursor += 1;
    }
    if cursor == digit_start {
        return None;
    }
    if bytes.get(cursor) == Some(&b';') {
        cursor += 1;
    }
    Some((cursor, decode_codepoint(value)?))
}

fn decode_codepoint(value: u32) -> Option<char> {
    const C1: [u32; 32] = [
        0x20ac, 0x81, 0x201a, 0x192, 0x201e, 0x2026, 0x2020, 0x2021, 0x2c6, 0x2030, 0x160, 0x2039,
        0x152, 0x8d, 0x17d, 0x8f, 0x90, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014,
        0x2dc, 0x2122, 0x161, 0x203a, 0x153, 0x9d, 0x17e, 0x178,
    ];
    let value = if (0x80..=0x9f).contains(&value) {
        C1[(value - 0x80) as usize]
    } else {
        value
    };
    match value {
        0 | 0xd800..=0xdfff | 0x110000.. => Some('\u{fffd}'),
        value => char::from_u32(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_parent(document: &Document, child: &str, parent: &str) {
        assert_eq!(
            document.parent_of(document.query_selector(child).unwrap()),
            document.query_selector(parent),
            "{child} should be inside {parent}"
        );
    }

    #[test]
    fn named_reference_table_is_complete_sorted_and_decodes_every_entry() {
        assert_eq!(entities::NAMED.len(), 2231);
        for pair in entities::NAMED.windows(2) {
            assert!(pair[0].0 < pair[1].0);
        }
        for &(name, expected) in entities::NAMED {
            assert!(name.len() <= 32);
            assert_eq!(decode_entities(&format!("&{name}")), expected, "{name}");
            assert_eq!(
                decode_entities_in_context(&format!("&{name}"), true),
                expected,
                "attribute {name}"
            );
        }
    }

    #[test]
    fn named_references_are_case_sensitive_longest_match_and_context_aware() {
        assert_eq!(
            decode_entities("&Aacute; &aacute; &NotEqualTilde; &AMP; &Amp;"),
            "Á á ≂\u{338} & &Amp;"
        );
        assert_eq!(
            decode_entities("&notin; &notit; &copy! &amp;lt;"),
            "∉ ¬it; ©! &lt;"
        );
        assert_eq!(
            decode_entities_in_context("&notit; &copy=1 &copy! &amp;lt;", true),
            "&notit; &copy=1 ©! &lt;"
        );
        assert_eq!(
            decode_entities("&unknown &copy; &é &amp;"),
            "&unknown © &é &"
        );
    }

    #[test]
    fn numeric_references_allow_missing_semicolons_and_replace_invalid_values() {
        assert_eq!(
            decode_entities("&#65 &#x41! &#X1f980; &#128; &#x9f;"),
            "A A! 🦀 € Ÿ"
        );
        assert_eq!(
            decode_entities("&#0; &#xD800; &#1114112; &#999999999999999999999;"),
            "� � � �"
        );
        assert_eq!(
            decode_entities("&#+65; &#-1; &#x; &#;"),
            "&#+65; &#-1; &#x; &#;"
        );
        assert_eq!(decode_entities(&format!("&#{};", "9".repeat(10000))), "�");
    }

    #[test]
    fn entity_context_is_used_in_attributes_and_rcdata() {
        let document = parse_document("<div title='&copy=1 &Aacute; &NotEqualTilde;'></div><textarea>&copy=1 &Aacute; &NotEqualTilde;</textarea>").unwrap();
        let Some(crate::dom::Node::Element(element)) =
            document.node(document.query_selector("div").unwrap())
        else {
            panic!("missing div");
        };
        assert_eq!(element.attr("title"), Some("&copy=1 Á ≂\u{338}"));
        assert_eq!(element_text_node(&document, "textarea"), "©=1 Á ≂\u{338}");
    }

    #[test]
    fn start_tag_slashes_delimit_attributes_but_not_unquoted_values() {
        let document = parse_document("<main><div/class='card'/id='box'><span>inside</span></div><a href=https://example.org/a/b/>link</a></main>").unwrap();
        assert_parent(&document, "span", "div");
        let Some(crate::dom::Node::Element(div)) =
            document.node(document.query_selector("div").unwrap())
        else {
            panic!("div missing");
        };
        assert_eq!(div.attr("class"), Some("card"));
        assert_eq!(div.attr("id"), Some("box"));
        let Some(crate::dom::Node::Element(link)) =
            document.node(document.query_selector("a").unwrap())
        else {
            panic!("link missing");
        };
        assert_eq!(link.attr("href"), Some("https://example.org/a/b/"));
    }

    #[test]
    fn non_html_whitespace_is_preserved_in_tag_and_attribute_names() {
        for space in ['\u{00a0}', '\u{2003}', '\u{000b}'] {
            let (tag, attrs) =
                parse_start_tag(&format!("div{space}x data{space}='value' title=a{space}b"));
            assert_eq!(tag, format!("div{space}x"));
            assert_eq!(
                attrs.get(&format!("data{space}")).map(String::as_str),
                Some("value")
            );
            assert_eq!(attrs.get("title"), Some(&format!("a{space}b")));
        }
    }

    #[test]
    fn end_tag_slash_and_attributes_close_the_matching_element() {
        for end in [
            "</div/>",
            "</DIV/>",
            "</div data-x='>'>",
            "</div/data-x='>'>",
            "</div\u{000c}>",
        ] {
            let document = parse_document(&format!(
                "<main><div><span>inside</span>{end}<p>after</p></main>"
            ))
            .unwrap();
            assert_parent(&document, "span", "div");
            assert_parent(&document, "p", "main");
        }
    }

    #[test]
    fn unicode_spaces_and_name_prefixes_do_not_close_another_tag() {
        for end in [
            "</div-extra>",
            "</div\u{00a0}>",
            "</div\u{2003}>",
            "</div\u{000b}>",
        ] {
            let document = parse_document(&format!(
                "<main><div>{end}<span>inside</span></div><p>after</p></main>"
            ))
            .unwrap();
            assert_parent(&document, "span", "div");
            assert_parent(&document, "p", "main");
        }
    }

    #[test]
    fn html_nonvoid_trailing_slash_does_not_close_element() {
        let document = parse_document(
            "<main><div/><span>inside</span></div><p>after</p><input/><img/><hr></main>",
        )
        .unwrap();
        assert_parent(&document, "span", "div");
        for tag in ["div", "p", "input", "img", "hr"] {
            assert_parent(&document, tag, "main");
        }
    }

    #[test]
    fn foreign_self_closing_elements_remain_siblings() {
        let document = parse_document("<main><svg><g/><rect/><title/><circle/></svg><math><mi/><mo/></math><p>after</p></main>").unwrap();
        for tag in ["g", "rect", "title", "circle"] {
            assert_parent(&document, tag, "svg");
        }
        for tag in ["mi", "mo"] {
            assert_parent(&document, tag, "math");
        }
        assert_parent(&document, "p", "main");
    }

    #[test]
    fn svg_integration_points_restore_html_parsing() {
        for tag in ["foreignObject", "desc", "title"] {
            let document = parse_document(&format!("<svg><{tag}><div/><span>inside</span></div><svg><path/><circle/></svg></{tag}><rect/></svg>")).unwrap();
            assert_parent(&document, "span", "div");
            assert_parent(&document, "div", &tag.to_ascii_lowercase());
            assert_eq!(
                document.parent_of(document.query_selector("path").unwrap()),
                document.parent_of(document.query_selector("circle").unwrap())
            );
            assert_parent(&document, "rect", "svg");
        }
    }

    #[test]
    fn mathml_integration_points_and_exceptions() {
        for parent in [
            "mtext",
            "annotation-xml encoding='TEXT/HTML'",
            "annotation-xml encoding='application/xhtml+xml'",
        ] {
            let tag = parent.split_whitespace().next().unwrap();
            let document = parse_document(&format!(
                "<math><{parent}><div/><span>x</span></div></{tag}><mo/></math>"
            ))
            .unwrap();
            assert_parent(&document, "span", "div");
            assert_parent(&document, "div", tag);
            assert_parent(&document, "mo", "math");
        }
        let document = parse_document("<math><mtext><mglyph/><malignmark/></mtext><annotation-xml><svg><path/><circle/></svg></annotation-xml></math>").unwrap();
        assert_parent(&document, "mglyph", "mtext");
        assert_parent(&document, "malignmark", "mtext");
        assert_parent(&document, "path", "svg");
        assert_parent(&document, "circle", "svg");
    }

    fn element_text_node<'a>(document: &'a Document, tag: &str) -> &'a str {
        let id = document.query_selector(tag).unwrap();
        let children = document.children(id);
        assert_eq!(children.len(), 1);
        match document.node(children[0]) {
            Some(crate::dom::Node::Text(text)) => text,
            _ => panic!("expected a text node"),
        }
    }

    #[test]
    fn rcdata_preserves_markup_as_text_and_decodes_entities_once() {
        for tag in ["textarea", "title"] {
            let html = format!(
                "<{tag}><strong>A &amp; B</strong><!--literal-->&amp;lt;</{tag}><p>after</p>"
            );
            let document = parse_document(&html).unwrap();
            assert!(document.query_selector("strong").is_none());
            assert_eq!(
                element_text_node(&document, tag),
                "<strong>A & B</strong><!--literal-->&lt;"
            );
            assert!(document.query_selector("p").is_some());
        }
    }

    #[test]
    fn textarea_ignores_only_the_first_line_feed() {
        for newline in ["\n", "\r\n", "\r", "&#10;"] {
            let document =
                parse_document(&format!("<textarea>{newline}\n  indented\n</textarea>")).unwrap();
            let expected = if newline == "\r" {
                "  indented\n"
            } else {
                "\n  indented\n"
            };
            assert_eq!(element_text_node(&document, "textarea"), expected);
        }
        let document = parse_document("<title>\nHeading</title>").unwrap();
        assert_eq!(element_text_node(&document, "title"), "\nHeading");
    }

    #[test]
    fn rcdata_end_tags_match_names_not_prefixes_or_encoded_markup() {
        let document =
            parse_document("<textarea></textareax>&lt;/textarea&gt;</TeXtArEa \n><p>after</p>")
                .unwrap();
        assert_eq!(
            element_text_node(&document, "textarea"),
            "</textareax></textarea>"
        );
        assert!(document.query_selector("p").is_some());
    }

    #[test]
    fn unterminated_rcdata_consumes_the_remaining_source_as_text() {
        for tag in ["textarea", "title"] {
            let document =
                parse_document(&format!("<{tag}>text <p>not an element</p> &amp;")).unwrap();
            assert_eq!(
                element_text_node(&document, tag),
                "text <p>not an element</p> &"
            );
            assert!(document.query_selector("p").is_none());
        }
    }

    #[test]
    fn quoted_attribute_values_can_contain_tag_delimiters() {
        let document = parse_document(r#"<div title="a > b < c" data-label='x > y' data-note="l'été">visible</div><p>after</p>"#).unwrap();
        let id = document.query_selector("div").unwrap();
        let Some(crate::dom::Node::Element(element)) = document.node(id) else {
            panic!("element")
        };
        assert_eq!(element.attr("title"), Some("a > b < c"));
        assert_eq!(element.attr("data-label"), Some("x > y"));
        assert_eq!(element.attr("data-note"), Some("l'été"));
        assert_eq!(document.text_content(id), "visible");
        assert!(document.query_selector("p").is_some());
    }

    #[test]
    fn trailing_slash_in_unquoted_value_is_not_self_closing_markup() {
        let document = parse_document("<div data-url=https://example.test/>inside</div>").unwrap();
        let id = document.query_selector("div").unwrap();
        let Some(crate::dom::Node::Element(element)) = document.node(id) else {
            panic!("element")
        };
        assert_eq!(element.attr("data-url"), Some("https://example.test/"));
        assert_eq!(document.text_content(id), "inside");
    }

    #[test]
    fn raw_text_end_tags_allow_whitespace_and_ignore_name_prefixes() {
        let document =
            parse_document("<style>.x::before {content:'</stylex>';}</STYLE \n><p>visible</p>")
                .unwrap();
        let id = document.query_selector("style").unwrap();
        let child = document.children(id)[0];
        assert!(
            matches!(document.node(child), Some(crate::dom::Node::Text(text)) if text == ".x::before {content:'</stylex>';}" )
        );
        assert_eq!(
            document.text_content(document.query_selector("p").unwrap()),
            "visible"
        );
    }

    #[test]
    fn unterminated_raw_text_does_not_create_visible_elements() {
        for tag in ["style", "script"] {
            let document =
                parse_document(&format!("<{tag}>ignored <p>not an element</p>")).unwrap();
            assert!(document.query_selector("p").is_none());
            let id = document.query_selector(tag).unwrap();
            assert_eq!(document.children(id).len(), 1);
        }
    }

    #[test]
    fn unmatched_end_tag_does_not_pop_the_open_parent() {
        let document = parse_document("<div></unknown><span>inside</span></div>").unwrap();
        let div = document.query_selector("div").unwrap();
        let span = document.query_selector("span").unwrap();
        assert_eq!(document.parent_of(span), Some(div));
    }

    #[test]
    fn first_duplicate_attribute_wins() {
        let document = parse_document("<div id=first ID=second></div>").unwrap();
        assert!(document.query_selector("#first").is_some());
        assert!(document.query_selector("#second").is_none());
    }

    #[test]
    fn less_than_without_a_tag_name_is_text() {
        let document = parse_document("<p>1 < 2 and 3 > 2</p>").unwrap();
        let p = document.query_selector("p").unwrap();
        assert_eq!(document.text_content(p), "1 < 2 and 3 > 2");
    }

    #[test]
    fn quote_inside_unquoted_value_does_not_swallow_following_markup() {
        let document = parse_document("<div title=don't>first</div><p>second</p>").unwrap();
        let div = document.query_selector("div").unwrap();
        let Some(crate::dom::Node::Element(element)) = document.node(div) else {
            panic!("element")
        };
        assert_eq!(element.attr("title"), Some("don't"));
        assert_eq!(
            document.text_content(document.query_selector("p").unwrap()),
            "second"
        );
    }

    #[test]
    fn incomplete_quoted_tag_is_not_emitted_as_visible_text() {
        let document = parse_document("<p>before</p><div title='unterminated > attribute").unwrap();
        assert!(document.query_selector("div").is_none());
        assert_eq!(document.text_content(document.root()), "before");
    }

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
