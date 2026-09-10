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
    def gate(self, counts, errors, *, mismatched_page=None, pages="all", max_pages=None):
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
            if mismatched_page is not None:
                metrics[mismatched_page]["browser_size"] = [200, 100]
                metrics[mismatched_page]["browser_raster_was_resized_for_metric"] = True
            extra_args = [] if max_pages is None else ["--max-pages", str(max_pages)]
            with patch("sys.argv", ["audit", str(html), "--out-dir", str(root),
                                    "--pages", pages, "--threshold-rmse-normalized", "0.1"] + extra_args), \
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

    def test_resized_later_page_cannot_pass_even_with_zero_pixel_error(self):
        self.assertEqual(self.gate([2, 2], [0.0, 0.0], mismatched_page=1), 1)

    def test_first_page_only_is_not_a_complete_multipage_gate(self):
        self.assertEqual(self.gate([2, 2], [0.0], pages="first"), 1)

    def test_page_cap_cannot_hide_uncompared_pages(self):
        self.assertEqual(self.gate([3, 3], [0.0, 0.0], max_pages=2), 1)

    def test_first_page_mode_can_validate_a_single_page_document(self):
        self.assertEqual(self.gate([1, 1], [0.0], pages="first"), 0)

    def test_empty_pdf_is_rejected(self):
        self.assertEqual(self.gate([0, 1], []), 2)

    def test_scientific_notation_metrics_are_supported(self):
        self.assertEqual(parity.parse_rmse("0.65535 (1e-5)\n"), (0.65535, 0.00001))

    def test_invalid_or_ambiguous_metrics_are_rejected(self):
        for output in ["1e999 (0.1)", "12 (1.2)", "warning 0 (0)", "0 (0)\n1 (0.1)"]:
            with self.subTest(output=output), self.assertRaises(ValueError):
                parity.parse_rmse(output)

    def test_unknown_metric_is_not_perfect_match(self):
        with self.assertRaises(ValueError):
            parity.parse_rmse("unexpected tool output")


if __name__ == "__main__":
    unittest.main()
