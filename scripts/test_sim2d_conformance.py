"""Cost regressions must survive record boundaries and retain their comparator."""
import unittest
import json
import tempfile
from pathlib import Path
from contextlib import redirect_stderr
from io import StringIO
from copy import deepcopy
from unittest.mock import patch

from sim2d_conformance import ROOT, Runner, SIZES, build_settings, control, last_comparable, load_cost_record, main, measure, record_trace, registration, run, runner_graph


class RegisteredRules(unittest.TestCase):
    def test_other_rules_choose_traces_sizes_vectors_and_witnesses(self):
        config = {"runner": "other-run", "sizes": [[3, 9]], "vectors": "other-vectors.json",
                  "witnesses": ["changed_rewinds"], "traces": [
                      {"name": "other", "retained": "other-retained.txt", "variant": False,
                       "witnesses": ["other_hits"]}]}
        runner = unittest.mock.Mock()
        runner.artifact_hash, runner.device, runner.user_agent = "artifact", None, "Chrome"
        runner.invoke_many.return_value = [[17], {"other_hits": 2}, {"wire": True}]
        with patch("sim2d_conformance.registration", return_value=config), \
                patch("sim2d_conformance.SIZES", [(3, 9)]), \
                patch("sim2d_conformance.Runner", return_value=runner), \
                patch("sim2d_conformance.source_key", return_value="source"), \
                patch("sim2d_conformance.references", return_value={}) as retained, \
                patch("sim2d_conformance.compare"), \
                patch.object(Path, "read_text", return_value="[17]"):
            result = run("chrome-webgpu")
        self.assertEqual(len(result["rows"]), 2)
        self.assertEqual(runner.invoke_many.call_args.args[0], [
            ("vectors", []), ("trace", [3, 9, False, False, False, False, False]),
            ("trace", [3, 9, False, True, False, False, False])])
        retained.assert_called_once_with(ROOT / "other-retained.txt")

    def test_control_uses_registered_adapter_and_counts(self):
        mac, chrome = unittest.mock.Mock(), unittest.mock.Mock()
        mac.invoke.return_value = {"fold": "a", "timeline": ["a"]}
        chrome.invoke.return_value = {"fold": "b", "timeline": ["b"]}
        chrome.user_agent = "Chrome"
        with patch("sim2d_conformance.registration", return_value={"control": {"bodies": 7, "ticks": 1}}), \
                patch("sim2d_conformance.Runner", side_effect=[mac, chrome]):
            result = control("chrome-webgpu")
        mac.invoke.assert_called_once_with("conformance", [7, 1, False, False, True])
        chrome.invoke.assert_called_once_with("conformance", [7, 1, False, False, True])
        self.assertEqual(result["first_differing_tick"], 1)

    def test_record_cannot_overwrite_retained_reference(self):
        with self.assertRaisesRegex(ValueError, "candidate"):
            record_trace("crowd", ROOT / "examples/crowd-2d/retained/crowd.txt")

    def test_non_rule_package_cannot_select_a_runner(self):
        with self.assertRaisesRegex(ValueError, "registered rule package"):
            registration("thrum-core")


class BrowserBatch(unittest.TestCase):
    def test_conformance_takes_one_queue_turn_for_every_retained_route(self):
        runner = Runner.__new__(Runner)
        runner.target = "chrome-webgpu"
        runner.program = "web-root"
        runner.artifact_hash = "artifact"
        runner.device = None
        rows = [{"timeline": [], "oversize": 1, "capsule_hits": 1, "swept_only_hits": 1}
                for _ in range(8)]
        packet = {"result": [json.loads((ROOT / "crates/thrum-core/src/profile/vectors.json").read_text()), *rows],
                  "userAgent": "Chrome", "crossOriginIsolated": True}
        with patch("sim2d_conformance.Runner", return_value=runner), \
                patch("sim2d_conformance.source_key", return_value="source"), \
                patch("sim2d_conformance.device_command", return_value=json.dumps(packet)) as device, \
                patch("sim2d_conformance.compare") as compare:
            result = run("chrome-webgpu")
        device.assert_called_once()
        resource, command = device.call_args.args
        self.assertEqual(resource, "chrome")
        requests = json.loads(command[-2])
        self.assertEqual(requests[0], {"method": "vectors", "args": [], "omitTimeline": True})
        self.assertTrue(all(request["omitTimeline"] for request in requests))
        expected = []
        for mixed in [False, True]:
            expected.extend([count, ticks, False, False, False, mixed, False] for count, ticks in SIZES)
            expected.append([2000, 3600, False, True, False, mixed, False])
        self.assertEqual([request["args"] for request in requests[1:]], expected)
        self.assertEqual(compare.call_count, 8)
        self.assertEqual(len(result["rows"]), 8)

    def test_missing_browser_result_is_refused(self):
        runner = Runner.__new__(Runner)
        runner.target = "chrome-webgpu"
        runner.program = "web-root"
        packet = {"result": [], "userAgent": "Chrome", "crossOriginIsolated": True}
        with patch("sim2d_conformance.device_command", return_value=json.dumps(packet)):
            with self.assertRaisesRegex(RuntimeError, "incomplete conformance batch"):
                runner.invoke_many([("vectors", [])])


class BuildIdentity(unittest.TestCase):
    def test_workspace_lock_feature_union_and_version_qualification_are_ignored(self):
        manifest = {"workspace": {"members": ["runner"], "resolver": "3"}}
        lock = {"version": 4, "package": [{"name": "futures-util", "version": "1", "checksum": "same",
                                           "dependencies": ["syn"]}]}
        selected = {("futures-util", "1", None)}
        before = build_settings(manifest, lock, selected)
        lock["package"][0]["dependencies"] = ["syn 3.0.9", "futures-io", "futures-channel"]
        self.assertEqual(before, build_settings(manifest, lock, selected))

    def test_runner_features_and_active_edges_are_covered(self):
        packages = {name: {"id": name, "name": name, "version": "1", "source": None}
                    for name in ["runner", "a", "b", "unrelated"]}
        tree = '0|runner v1|\n1|a v1|std\n1|b v1|\n'
        selected, graph = runner_graph(tree, packages)
        self.assertEqual(selected, {"runner", "a", "b"})
        self.assertNotEqual(graph, runner_graph(tree.replace('|std', '|std,io'), packages)[1])
        self.assertNotEqual(graph, runner_graph(tree.replace('1|b', '2|b'), packages)[1])

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
    def test_validation_cannot_overwrite_measurement_with_receipt(self):
        destination = ROOT / "tests/fixtures/sim-2d/prior-costs-mac-metal.json"
        with patch("sys.argv", ["sim2d_conformance.py", "costs", "--output", str(destination)]), \
                patch("sim2d_conformance.identity", return_value={}), \
                patch("sim2d_conformance.costs") as validate, redirect_stderr(StringIO()):
            with self.assertRaises(SystemExit) as error:
                main()
        self.assertEqual(error.exception.code, 2)
        validate.assert_not_called()

    def test_receipt_is_refused_before_measurement_instead_of_keyerror(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "costs.json"
            path.write_text('{"record":"costs-mac-metal.json","timings_are_baselines":true}')
            with self.assertRaisesRegex(ValueError, "expected a measurement with runs"):
                load_cost_record(path)

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
                patch("sim2d_conformance.allocation_counts", return_value={}), \
                patch("sim2d_conformance.source_key", return_value="source"), \
                patch("sim2d_conformance.source_inputs", return_value={"fixture": "source"}), \
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
