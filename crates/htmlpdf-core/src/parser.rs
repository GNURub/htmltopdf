use crate::{dom::Document, RenderError};
use std::collections::BTreeMap;

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
            token.strip_suffix('/').unwrap_or(token).trim_end()
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
                } else if !byte.is_ascii_whitespace() {
                    state = State::Unquoted;
                }
            }
            State::Unquoted => {
                if byte == b'>' {
                    return Some((start + offset, false));
                }
                if byte.is_ascii_whitespace() {
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
                .is_some_and(|byte| byte.is_ascii_whitespace() || matches!(byte, b'>' | b'/'))
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
            attrs.entry(key).or_insert(value);
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

    fn assert_parent(document: &Document, child: &str, parent: &str) {
        assert_eq!(
            document.parent_of(document.query_selector(child).unwrap()),
            document.query_selector(parent),
            "{child} should be inside {parent}"
        );
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
