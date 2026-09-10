# HTML parser validation

The parser now scans tag boundaries with explicit quoted/unquoted attribute
states. A `>` inside a quoted value no longer ends the tag. Quotes appearing
inside unquoted values do not consume subsequent markup, and a trailing slash
inside an unquoted URL remains part of that value.

Raw script/style content is consumed through a matching closing name, allowing
case differences and whitespace before `>`. A name prefix such as `</stylex>`
does not close `<style>`. Unclosed raw-text elements consume the remaining source
instead of creating visible elements from it. The search avoids lowercasing a
copy of the entire remaining document.

Other corrections preserve the open parent when an unknown end tag appears,
keep the first duplicate attribute, treat `<` without a valid tag-name start as
text, and discard an incomplete tag at EOF rather than printing its attributes.

Nine regression tests verify these DOM and attribute behaviors. They are not a
browser screenshot comparison or proof of complete HTML parsing conformance.
Reference: [HTML parsing specification](https://html.spec.whatwg.org/multipage/parsing.html).

The parser remains incomplete: HTML insertion modes, implied elements and end
tags, adoption-agency recovery, full script escaped states,
and foreign-content namespace integration still require implementation and
conformance testing. Rendering equivalence and production readiness must not
be inferred from these tests alone.

## RCDATA and input newlines

`textarea` and `title` now collect their contents as text until their own end
tag. Embedded markup and comment syntax do not create child elements; character
references are decoded once, after locating the source end tag. An encoded end
tag therefore cannot terminate the element. Missing closing tags consume the
remaining input as text.

Input CRLF and bare CR are normalized to LF before tokenization, without copying
LF-only input. A textarea discards exactly its first LF, including when that LF
comes from a character reference; title does not discard its first LF. Four
additional tests cover these behaviors, case-insensitive end tags and name
prefixes. This verifies DOM text, not textarea visual fidelity.

## Textarea painting

Textarea values now come from untrimmed child text, not a `value` attribute.
An empty value may show its placeholder. The control has a separate multiline
painting path with preserved spaces and blank lines, soft word wrapping,
emergency splitting of oversized words, `wrap="off"`, and clipping inside the
control. Single-line input/button painting is unchanged. Three tests cover
value selection, whitespace-preserving wrapping, and line coordinates/clipping.

The complete `examples/generic-textarea.html` fixture was rendered and visually
inspected against Chromium headless shell revision 1243 using Poppler rasters.
Both PDFs have one page with 667 x 584 pixels without resizing. Normalized RMSE
is 0.162442; no acceptance threshold was asserted. Multiline content, blank
lines, indentation, literal markup and no-wrap clipping are visible. Chromium
also paints native scrollbars, which this engine does not yet implement; their
space changes wrapping. Native control padding, placeholder appearance, tabs,
text alignment, Unicode line-breaking rules and exact font metrics still need
work. This fixture does not prove control or general browser parity.
