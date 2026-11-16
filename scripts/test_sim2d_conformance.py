"""Cost regressions must survive record boundaries and retain their comparator."""
import unittest
from copy import deepcopy
from unittest.mock import patch

from sim2d_conformance import build_settings, last_comparable, measure


class BuildIdentity(unittest.TestCase):
    def test_unlinked_workspace_addition_preserves_identity(self):
        manifest = {"workspace": {"members": ["runner"], "resolver": "3"}}
        lock = {"version": 4, "package": [{"name": "runner", "version": "1"}]}
        selected = {("runner", "1", None)}
        before = build_settings(manifest, lock, selected)
        manifest["workspace"]["members"].append("rain-model")
        manifest["profile"] = {"test": {"package": {"rain-model": {"opt-level": 2}}}}
        lock["package"].append({"name": "rain-model", "version": "1"})
        self.assertEqual(before, build_settings(manifest, lock, selected))

    def test_relevant_build_and_lock_changes_invalidate_identity(self):
        manifest = {"workspace": {"members": ["runner"], "resolver": "3"},
                    "profile": {"release": {"opt-level": 3}}}
        lock = {"version": 4, "package": [{"name": "dep", "version": "1", "source": "registry", "checksum": "a"}]}
        selected = {("dep", "1", "registry")}
        before = build_settings(manifest, lock, selected)
        changed = deepcopy(lock)
        changed["package"][0]["checksum"] = "b"
        self.assertNotEqual(before, build_settings(manifest, changed, selected))
        changed = deepcopy(manifest)
        changed["profile"]["release"]["opt-level"] = 2
        self.assertNotEqual(before, build_settings(changed, lock, selected))
        changed = deepcopy(manifest)
        changed["profile"]["release"]["package"] = {"dep": {"opt-level": 1}}
        self.assertNotEqual(before, build_settings(changed, lock, selected))
        changed["profile"]["release"]["package"] = {"dep@1": {"opt-level": 1}}
        self.assertNotEqual(before, build_settings(changed, lock, selected))


class CostComparisons(unittest.TestCase):
    def test_measure_keeps_previous_record_for_both_repeats(self):
        previous = {"commit": "previous", "runs": [{"repeat": 2, "rows": [
            {"bodies": count, "neighbours_per_tick": count, "tick_median_ms": 1.0}
            for count in (2000, 5000, 10000)
        ]}]}

        def timing(_method, args):
            count = args[0]
            ticks = 36000 if count == 10000 else 3600
            return {"bodies": count, "ticks": ticks, "samples": ticks - 120,
                    "neighbours_per_tick": count, "tick_ms_per_neighbour": 1.5 / count,
                    **dict.fromkeys(["tick_median_ms", "index_median_ms", "digest_median_ms",
                                     "rewind_eight_ms", "replay_eight_ms"], 1.5)}

        with patch("sim2d_conformance.Runner") as runner, \
                patch("sim2d_conformance.source_key", return_value="source"), \
                patch("sim2d_conformance.command", return_value="current"), \
                patch("sim2d_conformance.json.loads", return_value=previous):
            runner.return_value.invoke.side_effect = timing
            record = measure("mac-metal")
        for run in record["runs"]:
            for row in run["rows"]:
                comparison = row["quarter_slower_comparisons"][0]
                self.assertEqual(comparison["origin"], "previous_record")
                self.assertEqual(comparison["commit"], "previous")
                self.assertTrue(comparison["tripped"])
        self.assertEqual(len(record["runs"][1]["rows"][0]["quarter_slower_comparisons"]), 2)

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
