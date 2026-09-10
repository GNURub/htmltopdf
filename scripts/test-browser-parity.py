"""Exercise the release gate without requiring a browser or PDF tools."""
import importlib.util
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "parity", Path(__file__).with_name("audit-browser-parity.py")
)
parity = importlib.util.module_from_spec(spec)
spec.loader.exec_module(parity)


class GateTests(unittest.TestCase):
    def gate(self, counts, errors):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            html = root / "input.html"
            html.touch()
            metrics = [dict(
                htmlpdf_png="actual.png", browser_png="reference.png",
                diff_png="diff.png", htmlpdf_size=[100, 100],
                browser_size=[100, 100], browser_raster_was_resized_for_metric=False,
                rmse_absolute=value * 65535, rmse_normalized=value,
            ) for value in errors]
            with patch("sys.argv", ["audit", str(html), "--out-dir", str(root),
                                    "--pages", "all", "--threshold-rmse-normalized", "0.1"]), \
                 patch.object(parity, "run"), patch.object(parity, "require_ok"), \
                 patch.object(parity, "pdf_page_count", side_effect=counts), \
                 patch.object(parity, "render_pdf_page"), \
                 patch.object(parity, "compare_pngs", side_effect=metrics), \
                 patch("builtins.print"):
                return parity.main()

    def test_later_page_failure_fails_process(self):
        self.assertEqual(self.gate([2, 2], [0.01, 0.3]), 1)

    def test_missing_pages_fail_even_when_pixels_match(self):
        self.assertEqual(self.gate([1, 2], [0.0]), 1)

    def test_matching_pages_pass(self):
        self.assertEqual(self.gate([2, 2], [0.01, 0.02]), 0)

    def test_unknown_metric_is_not_perfect_match(self):
        with self.assertRaises(ValueError):
            parity.parse_rmse("unexpected tool output")


if __name__ == "__main__":
    unittest.main()
