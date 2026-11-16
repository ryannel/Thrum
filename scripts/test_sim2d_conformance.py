"""Cost regressions must survive record boundaries and retain their comparator."""
import unittest

from sim2d_conformance import last_comparable


class CostComparisons(unittest.TestCase):
    def test_first_repeat_detects_review_regression_against_previous_record(self):
        record = {"commit": "previous", "runs": [
            {"repeat": 1, "rows": [{"bodies": 5000, "neighbours_per_tick": 65407, "tick_median_ms": 1.21}]},
            {"repeat": 2, "rows": [{"bodies": 5000, "neighbours_per_tick": 65407, "tick_median_ms": 1.19}]},
        ]}
        row = {"bodies": 5000, "neighbours_per_tick": 65407, "tick_median_ms": 1.751}
        comparison = last_comparable(row, record, "previous_record")
        self.assertTrue(comparison["tripped"])
        self.assertEqual(comparison["repeat"], 2)
        self.assertEqual(comparison["commit"], "previous")
        self.assertEqual(comparison["tick_median_ms"], 1.19)

    def test_skips_incomparable_density_and_size(self):
        record = {"commit": "previous", "runs": [
            {"repeat": 1, "rows": [{"bodies": 5000, "neighbours_per_tick": 100, "tick_median_ms": 2.0}]},
            {"repeat": 2, "rows": [
                {"bodies": 5000, "neighbours_per_tick": 200, "tick_median_ms": 1.0},
                {"bodies": 10000, "neighbours_per_tick": 100, "tick_median_ms": 1.0},
            ]},
        ]}
        row = {"bodies": 5000, "neighbours_per_tick": 100, "tick_median_ms": 2.5}
        comparison = last_comparable(row, record, "current_record")
        self.assertEqual(comparison["repeat"], 1)
        self.assertFalse(comparison["tripped"])
        row["neighbours_per_tick"] = 300
        self.assertIsNone(last_comparable(row, record, "previous_record"))
        self.assertIsNone(last_comparable(row, None, "previous_record"))


if __name__ == "__main__":
    unittest.main()
