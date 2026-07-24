#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
REGENERATE=0
REQUIRE_TARGET="${INVOICE_REQUIRE_TARGET:-0}"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --regenerate)
      REGENERATE=1
      ;;
    --require-target)
      REQUIRE_TARGET=1
      ;;
    *)
      echo "usage: $0 [--regenerate] [--require-target]" >&2
      exit 2
      ;;
  esac
  shift
done

BEFORE="$ROOT_DIR/docs/visual-regression/artifacts/invoice-before-broken.png"
CURRENT="$ROOT_DIR/docs/visual-regression/artifacts/invoice-current.png"
DIFF="$ROOT_DIR/docs/visual-regression/artifacts/invoice-current-vs-before-diff.png"
TARGET="${INVOICE_TARGET_PNG:-$ROOT_DIR/docs/visual-regression/artifacts/invoice-target.png}"
TARGET_DIFF="$ROOT_DIR/docs/visual-regression/artifacts/invoice-current-vs-target-diff.png"
TARGET_MAE_MAX="${INVOICE_TARGET_MAE_MAX:-0}"
PDF="$ROOT_DIR/out.pdf"

if ! command -v identify >/dev/null 2>&1; then
  echo "missing dependency: ImageMagick identify" >&2
  exit 1
fi

if ! command -v compare >/dev/null 2>&1; then
  echo "missing dependency: ImageMagick compare" >&2
  exit 1
fi

if ! command -v pdfinfo >/dev/null 2>&1; then
  echo "missing dependency: pdfinfo" >&2
  exit 1
fi

if [ "$REGENERATE" -eq 1 ]; then
  if ! command -v pdftoppm >/dev/null 2>&1; then
    echo "missing dependency: pdftoppm" >&2
    exit 1
  fi
  "$ROOT_DIR/scripts/render-invoice-preview.py"
  pdftoppm -png -f 1 -singlefile "$PDF" "$ROOT_DIR/docs/visual-regression/artifacts/invoice-current"
fi

if [ ! -s "$BEFORE" ]; then
  echo "missing broken baseline: $BEFORE" >&2
  exit 1
fi

if [ ! -s "$CURRENT" ]; then
  echo "missing current snapshot: $CURRENT" >&2
  exit 1
fi

if [ ! -s "$PDF" ]; then
  echo "missing PDF artifact: $PDF" >&2
  exit 1
fi

echo "== Image dimensions =="
identify "$BEFORE" "$CURRENT"

echo "== Image quality =="
"$ROOT_DIR/scripts/audit-invoice-image-quality.py"

echo "== PDF info =="
pdfinfo "$PDF" | sed -n '1,40p'

echo "== Difference from broken baseline =="
set +e
METRIC=$(compare -metric MAE -resize 1316x1147\! "$CURRENT" "$BEFORE" "$DIFF" 2>&1)
STATUS=$?
set -e
# ImageMagick compare exits 1 when images differ. For this check, difference is expected.
if [ "$STATUS" -gt 1 ]; then
  echo "$METRIC" >&2
  exit "$STATUS"
fi

echo "MAE $METRIC"
echo "diff written to $DIFF"

# Guard against accidentally reverting to the broken text-only output. The current
# rich snapshot should differ materially from the broken baseline.
VALUE=$(printf '%s' "$METRIC" | awk '{print $1}')
awk -v value="$VALUE" 'BEGIN { if (value < 10000) exit 1 }' || {
  echo "visual diff is too small; invoice may have regressed toward broken text-only output" >&2
  exit 1
}

echo "visual regression guard passed"

if [ -s "$TARGET" ]; then
  echo "== Difference from target reference =="
  CURRENT_DIMS=$(identify -format '%wx%h' "$CURRENT")
  TARGET_DIMS=$(identify -format '%wx%h' "$TARGET")
  if [ "$CURRENT_DIMS" != "$TARGET_DIMS" ]; then
    echo "target dimensions do not match current snapshot: target=$TARGET_DIMS current=$CURRENT_DIMS" >&2
    exit 1
  fi

  set +e
  TARGET_METRIC=$(compare -metric MAE "$CURRENT" "$TARGET" "$TARGET_DIFF" 2>&1)
  TARGET_STATUS=$?
  set -e
  if [ "$TARGET_STATUS" -gt 1 ]; then
    echo "$TARGET_METRIC" >&2
    exit "$TARGET_STATUS"
  fi

  echo "target MAE $TARGET_METRIC"
  echo "target diff written to $TARGET_DIFF"
  TARGET_VALUE=$(printf '%s' "$TARGET_METRIC" | awk '{print $1}')
  awk -v value="$TARGET_VALUE" -v max="$TARGET_MAE_MAX" 'BEGIN { if (value > max) exit 1 }' || {
    echo "target visual diff exceeds threshold: $TARGET_VALUE > $TARGET_MAE_MAX" >&2
    exit 1
  }
  echo "target visual match passed"
else
  echo "== Difference from target reference =="
  if [ "$REQUIRE_TARGET" -eq 1 ]; then
    echo "missing required target reference: $TARGET" >&2
    echo "Provide docs/visual-regression/artifacts/invoice-target.png or set INVOICE_TARGET_PNG=/path/to/target.png" >&2
    exit 1
  fi
  echo "skipped: no target reference at $TARGET"
fi

echo "== Preview/Rust parity =="
"$ROOT_DIR/scripts/audit-invoice-parity.py"

echo "== Layout geometry =="
"$ROOT_DIR/scripts/audit-invoice-layout-geometry.py"

echo "== PDF content audit =="
"$ROOT_DIR/scripts/audit-invoice-pdf-content.py"
