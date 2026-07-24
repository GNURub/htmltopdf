#!/usr/bin/env python3
"""Static smoke audit for portable font embedding support.

Font embedding is intentionally opt-in for now because the current layout metrics
are tuned for the existing default renderer. The audit verifies the capability
exists without making it a default visual-regression risk.
"""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
CHECKS = {
    "opt-in env flag": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "HTMLPDF_EMBED_FONTS"),
    "type0 font object": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "/Subtype /Type0"),
    "cid font object": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "/Subtype /CIDFontType2"),
    "identity encoding": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "/Encoding /Identity-H"),
    "to unicode map": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "/ToUnicode"),
    "cid to gid map": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "/CIDToGIDMap"),
    "font descriptor": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "/FontDescriptor"),
    "font file stream": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "/FontFile2"),
    "cid width array": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "/W ["),
    "layout width sharing": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "TrueTypeMetrics"),
    "noto sans candidate": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "NotoSans-Regular.ttf"),
    "lato candidate": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "Lato-Regular.ttf"),
}

failures = []
for label, (path, needle) in CHECKS.items():
    if not path.exists():
        failures.append(f"{label}: missing {path.relative_to(ROOT)}")
        continue
    if needle not in path.read_text(errors="ignore"):
        failures.append(f"{label}: missing {needle!r} in {path.relative_to(ROOT)}")

if failures:
    print("generic font support audit FAILED", file=sys.stderr)
    for failure in failures:
        print(f"- {failure}", file=sys.stderr)
    sys.exit(1)

print("generic font support audit passed")
