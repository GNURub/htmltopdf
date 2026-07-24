#!/usr/bin/env python3
"""Static smoke audit for generic JPEG <img> support.

This intentionally checks capabilities, not one customer fixture: the open-source
renderer must load local/data JPEG sources, lay them out as replaced elements, and
emit native PDF image XObjects.
"""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
CHECKS = {
    "layout image item": (ROOT / "crates/htmlpdf-core/src/layout.rs", "LayoutItem::Image"),
    "layout img element route": (ROOT / "crates/htmlpdf-core/src/layout.rs", 'element.tag == "img"'),
    "local image asset resolver": (ROOT / "crates/htmlpdf-core/src/layout.rs", "resolve_local_image_asset"),
    "data uri image decoder": (ROOT / "crates/htmlpdf-core/src/layout.rs", "decode_data_uri_image"),
    "jpeg dimension parser": (ROOT / "crates/htmlpdf-core/src/layout.rs", "jpeg_dimensions"),
    "png image data parser": (ROOT / "crates/htmlpdf-core/src/layout.rs", "png_image_data"),
    "object-fit parser": (ROOT / "crates/htmlpdf-core/src/css.rs", '"object-fit"'),
    "object-position parser": (ROOT / "crates/htmlpdf-core/src/css.rs", '"object-position"'),
    "object-position layout": (ROOT / "crates/htmlpdf-core/src/layout.rs", "style.object_position_x"),
    "pdf xobject resources": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "/XObject <<"),
    "pdf image subtype": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "/Subtype /Image"),
    "pdf dct jpeg filter": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "/DCTDecode"),
    "pdf flate png filter": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "/FlateDecode"),
    "pdf png predictor": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "/Predictor 15"),
    "pdf png alpha mask": (ROOT / "crates/htmlpdf-core/src/pdf.rs", "/SMask"),
    "png alpha decoder": (ROOT / "crates/htmlpdf-core/src/layout.rs", "defilter_png_scanlines"),
    "png palette parser": (ROOT / "crates/htmlpdf-core/src/layout.rs", 'b"PLTE"'),
    "png palette transparency parser": (ROOT / "crates/htmlpdf-core/src/layout.rs", 'b"tRNS"'),
    "css background image parser": (ROOT / "crates/htmlpdf-core/src/css.rs", "background_image"),
    "css background-size parser": (ROOT / "crates/htmlpdf-core/src/css.rs", "BackgroundSize::Cover"),
    "layout background image item": (ROOT / "crates/htmlpdf-core/src/layout.rs", "push_background_image_item"),
    "generic image fixture": (ROOT / "examples/generic-images.html", "object-fit: cover"),
    "generic object position fixture": (ROOT / "examples/generic-images.html", "object-position: right 25%"),
    "generic png fixture": (ROOT / "examples/generic-images.html", "generic-logo.png"),
    "generic png alpha fixture": (ROOT / "examples/generic-images.html", "generic-alpha.png"),
    "generic png palette fixture": (ROOT / "examples/generic-images.html", "generic-palette.png"),
    "generic background image fixture": (ROOT / "examples/generic-images.html", "background-image: url"),
    "local jpeg fixture": (ROOT / "examples/assets/generic-photo.jpg", b"\xff\xd8"),
    "local png fixture": (ROOT / "examples/assets/generic-logo.png", b"\x89PNG\r\n\x1a\n"),
    "local png alpha fixture": (ROOT / "examples/assets/generic-alpha.png", b"\x89PNG\r\n\x1a\n"),
    "local png palette fixture": (ROOT / "examples/assets/generic-palette.png", b"\x89PNG\r\n\x1a\n"),
}

failures = []
for label, (path, needle) in CHECKS.items():
    if not path.exists():
        failures.append(f"{label}: missing {path.relative_to(ROOT)}")
        continue
    if isinstance(needle, bytes):
        data = path.read_bytes()
        ok = data.startswith(needle)
    else:
        data = path.read_text(errors="ignore")
        ok = needle in data
    if not ok:
        failures.append(f"{label}: missing {needle!r} in {path.relative_to(ROOT)}")

if failures:
    print("generic image support audit FAILED", file=sys.stderr)
    for failure in failures:
        print(f"- {failure}", file=sys.stderr)
    sys.exit(1)

print("generic image support audit passed")
