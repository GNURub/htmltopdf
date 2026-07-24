use crate::{dom::Document, RenderError};
use std::time::{Duration, Instant};

/// Execute the v1 deterministic JavaScript subset.
///
/// Supported statements:
/// - `document.querySelector("#id").textContent = "value";`
/// - `htmlpdf.setText("#id", "value");`
///
/// This is intentionally tiny. It gives template authors a migration path for
/// simple DOM/data preparation without embedding a whole browser runtime.
pub fn run_limited_scripts(document: &mut Document, timeout_ms: u64) -> Result<(), RenderError> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms.max(1));
    for script in document.scripts() {
        for statement in script.split(';') {
            if Instant::now() > deadline {
                return Err(RenderError::JavaScript(format!(
                    "limited JS exceeded {timeout_ms}ms timeout"
                )));
            }
            let statement = statement.trim();
            if statement.is_empty() {
                continue;
            }
            execute_statement(document, statement)?;
        }
    }
    Ok(())
}

fn execute_statement(document: &mut Document, statement: &str) -> Result<(), RenderError> {
    if statement.starts_with("document.querySelector") {
        return execute_query_selector_assignment(document, statement);
    }
    if statement.starts_with("htmlpdf.setText") {
        return execute_set_text(document, statement);
    }

    Err(RenderError::JavaScript(format!(
        "unsupported limited-JS statement: {statement}"
    )))
}

fn execute_query_selector_assignment(
    document: &mut Document,
    statement: &str,
) -> Result<(), RenderError> {
    let selector_start = statement.find('(').ok_or_else(|| malformed(statement))? + 1;
    let selector_end =
        find_matching_paren(statement, selector_start - 1).ok_or_else(|| malformed(statement))?;
    let selector = parse_string_literal(statement[selector_start..selector_end].trim())?;
    let remainder = statement[selector_end + 1..].trim();
    let Some(remainder) = remainder.strip_prefix(".textContent") else {
        return Err(RenderError::JavaScript(
            "only .textContent assignment is supported after querySelector".to_string(),
        ));
    };
    let Some((_, value)) = remainder.split_once('=') else {
        return Err(malformed(statement));
    };
    let value = parse_string_literal(value.trim())?;
    set_text(document, &selector, value)
}

fn execute_set_text(document: &mut Document, statement: &str) -> Result<(), RenderError> {
    let open = statement.find('(').ok_or_else(|| malformed(statement))?;
    let close = find_matching_paren(statement, open).ok_or_else(|| malformed(statement))?;
    let args = split_args(&statement[open + 1..close]);
    if args.len() != 2 {
        return Err(RenderError::JavaScript(
            "htmlpdf.setText expects exactly 2 string arguments".to_string(),
        ));
    }
    let selector = parse_string_literal(args[0].trim())?;
    let value = parse_string_literal(args[1].trim())?;
    set_text(document, &selector, value)
}

fn set_text(document: &mut Document, selector: &str, value: String) -> Result<(), RenderError> {
    let Some(node) = document.query_selector(selector) else {
        return Err(RenderError::JavaScript(format!(
            "selector did not match any element: {selector}"
        )));
    };
    document.set_text_content(node, value);
    Ok(())
}

fn parse_string_literal(input: &str) -> Result<String, RenderError> {
    let bytes = input.as_bytes();
    if bytes.len() < 2 || (bytes[0] != b'\'' && bytes[0] != b'"') {
        return Err(RenderError::JavaScript(format!(
            "expected string literal, got: {input}"
        )));
    }
    let quote = bytes[0];
    let mut out = String::new();
    let mut escaped = false;
    for (idx, ch) in input[1..].char_indices() {
        let absolute_idx = idx + 1;
        if escaped {
            match ch {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                '\\' => out.push('\\'),
                '\'' => out.push('\''),
                '"' => out.push('"'),
                other => out.push(other),
            }
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if input.as_bytes().get(absolute_idx) == Some(&quote) {
            if input[absolute_idx + 1..].trim().is_empty() {
                return Ok(out);
            }
            return Err(RenderError::JavaScript(format!(
                "unexpected characters after string literal: {}",
                &input[absolute_idx + 1..]
            )));
        }
        out.push(ch);
    }
    Err(RenderError::JavaScript(
        "unterminated string literal".to_string(),
    ))
}

fn find_matching_paren(input: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (idx, ch) in input.char_indices().skip_while(|(idx, _)| *idx < open) {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active_quote {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_args(input: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut start = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (idx, ch) in input.char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active_quote {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            ',' => {
                args.push(input[start..idx].trim());
                start = idx + 1;
            }
            _ => {}
        }
    }
    args.push(input[start..].trim());
    args
}

fn malformed(statement: &str) -> RenderError {
    RenderError::JavaScript(format!("malformed limited-JS statement: {statement}"))
}
