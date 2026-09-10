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
space changes wrapping. Native control padding, placeholder appearance,
text alignment, Unicode line-breaking rules and exact font metrics still need
work. This fixture does not prove control or general browser parity.

Textarea wrapping now stops after enough lines have been generated to cover the
visible control height (plus a rounding guard line). It does not build the full
list of off-screen wrapped lines. Prefix-equivalence tests cover blank lines,
spaces, Unicode, narrow widths and no-wrap mode; a 100,000-line input checks the
bounded output count. This bounds the number of generated lines, not all memory
use: source text, text transformation, and a single very long unwrapped line
still require separate resource-limit work. No new visual score is claimed.

Preserved textarea tabs now advance to the next stop instead of being passed to
the PDF writer as control characters. `tab-size` accepts nonnegative numbers
(multiples of the space advance) and lengths, inherits, and defaults to eight
spaces. Zero disables the advance; negative, non-finite and percentage values
are ignored. Wrapping and text-run placement share the stop calculation.
Three regression tests cover column coordinates, line reset, soft wrapping,
zero advance, parsing and inheritance. This is geometry validation, not a new
visual score, and applies to textarea painting rather than all preformatted
inline contexts. Reference: [CSS tab sizing](https://www.w3.org/TR/css-text-3/#tab-size-property).

### HTML versus foreign-content self-closing tags

Trailing `/>` no longer closes ordinary non-void HTML elements. HTML void
elements still close immediately, while SVG and MathML retain self-closing
behavior. Parser-local namespace tracking handles SVG `foreignObject`, `desc`
and `title`, MathML text integration points (with `mglyph`/`malignmark`
exceptions), and HTML-encoded `annotation-xml`. SVG `title` is no longer
consumed as HTML RCDATA. Four structural regression tests cover these cases,
including nested SVG and following siblings. No runtime dependency was added.

This is not a full HTML tree-construction implementation: foreign-content
breakout rules, implied elements/end tags, adoption-agency handling and
namespace-aware DOM consumers remain incomplete. The checks establish tree
relationships, not browser visual parity. Reference:
[WHATWG HTML parsing](https://html.spec.whatwg.org/multipage/parsing.html).

End-tag name recognition now treats a slash as a delimiter, so `</div/>` and
end tags with ignored attributes close their matching element. Token trimming
and end-tag name splitting use HTML whitespace rather than Unicode whitespace;
NBSP, em space and vertical tab no longer close an unrelated plain tag name.
Two structural tests cover slash/attribute variants, case folding, form feed,
Unicode lookalikes and following-sibling parentage. This is not complete HTML
tokenizer conformance: start-tag/attribute whitespace handling and malformed
tree-construction recovery still require further work.

Start-tag name and attribute parsing now share HTML whitespace boundaries too.
A slash separates the tag name and attributes, or successive attribute names,
without truncating an unquoted URL value. Unicode whitespace and vertical tab
remain part of names/values instead of being silently trimmed or split.
Tag-boundary scanning, self-closing cleanup and raw-text end detection use the
same whitespace predicate. Two regressions cover attribute slash recovery,
URL preservation, child parentage and non-HTML whitespace in names and values.
These corrections still do not provide the complete HTML tokenizer or tree
construction algorithm.

### Complete named-reference table and contextual decoding

The parser now includes the 2,231 forms from the
[WHATWG entity table](https://html.spec.whatwg.org/entities.json), retrieved
2026-09-10, in a sorted static Rust table. No runtime dependency or download is
required. Names are case-sensitive, longest matching names win, and replacements
can contain two Unicode scalars. Legacy semicolonless names use the distinct
attribute ambiguity rule; text and RCDATA use the text rule. Decoding is one
pass and does not decode the replacement a second time.

Numeric references accept optional semicolons, decimal and hexadecimal digits,
replace invalid scalar values and overflow with U+FFFD, and apply the specified
C1 compatibility mapping. Named lookup examines at most 32 ASCII bytes after
an ampersand, rather than repeatedly scanning the rest of the input for `;`.
Tests exercise every table entry in both contexts, sorting/length invariants,
case differences, two-scalar replacements, ambiguous attributes, raw ampersands,
numeric errors and a 10,000-digit reference. DOM tests cover actual attributes
and textarea RCDATA. This improves decoding only: font coverage, Unicode
shaping, line breaking and the rest of HTML tree construction remain separate
requirements; no new visual parity score is claimed.

### Omitted list-item end tags

HTML `li` start tags now close the preceding open list item where the
special-element boundary permits it. `dt` and `dd` similarly close the previous
description item. Three structural regressions cover siblings, intervening
inline/div descendants, nested ordered/unordered lists and alternating
description terms/definitions. Foreign namespaces stop the lookup, and this
rule is not applied to foreign elements named `li`/`dt`/`dd`.

This implements list-item closure, not the complete in-body insertion mode.
Paragraph closure, formatting-element reconstruction, implied document
elements, table insertion modes and malformed foreign-content recovery remain
incomplete. These are DOM-structure tests, not list typography or visual parity
measurements. Reference: [HTML tree construction](https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inbody).

Paragraphs now close on the relevant HTML block start tags (including another
paragraph, headings and list items) when a paragraph is in button scope.
Two structural regressions cover eight block starts, intervening inline nodes,
list-item transitions and button/table/object scope boundaries. Closure is
driven by HTML tag semantics, not by the element's CSS display property.
Foreign namespaces conservatively stop the search. Quirks-dependent table
start handling, stray paragraph end-tag recovery and active-formatting
reconstruction remain incomplete; no visual parity result is claimed here.
