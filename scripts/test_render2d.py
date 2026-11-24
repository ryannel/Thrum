"""Sprite cost and window proof checks."""

from contextlib import redirect_stdout
import gzip
from io import StringIO
import json
from pathlib import Path
from subprocess import CompletedProcess
from tempfile import TemporaryDirectory
import unittest

from render2d import FIXTURES, TARGETS, validate
from render2d_crisp import occluded_transition


def retained(target):
    with gzip.open(FIXTURES / f"cost-{target}.json.gz", "rt") as receipt:
        return json.load(receipt)


class CostRecord(unittest.TestCase):
    def test_requires_build_and_compiler(self):
        for target in TARGETS:
            for field in ("build", "toolchain"):
                with self.subTest(target=target, field=field):
                    record = retained(target)
                    del record[field]
                    with self.assertRaisesRegex(ValueError, "release build identity"):
                        validate(record, target)

    def test_metal_requires_each_target_size(self):
        for field in ("scene_bytes_per_slot", "sdr_bytes_per_slot", "present_bytes"):
            with self.subTest(field=field):
                record = retained("mac-metal")
                del record["runs"][1][1]["storage"][field]
                with self.assertRaisesRegex(ValueError, "target storage bytes"):
                    validate(record, "mac-metal")

    def test_chrome_requires_observed_load_at_each_end_of_a_take(self):
        for edge in ("before", "after"):
            with self.subTest(edge=edge):
                record = retained("chrome-webgpu")
                record["runs"][1][1]["load"][edge] = None
                with self.assertRaisesRegex(ValueError, "machine load observation"):
                    validate(record, "chrome-webgpu")

    def test_recorded_timing_excursions_are_reported_without_failing(self):
        for target in TARGETS:
            with self.subTest(target=target):
                record = retained(target)
                for sample in record["runs"][1][1]["samples"]:
                    sample["render_cpu_ns"] = 1_000_000_000
                report = StringIO()
                with redirect_stdout(report):
                    validate(record, target)
                self.assertIn("second take render_cpu_ns", report.getvalue())


class OccludedWindow(unittest.TestCase):
    def test_geometry_requires_observed_occlusion_on_both_extents_and_no_drawable(self):
        with TemporaryDirectory() as scratch:
            arm = Path(scratch)
            log = arm / "game.log"
            summary = (
                "v1-game: 90 presented frames unavailable: presented=0, tick=30, "
                "display_link_callbacks=62, render_callbacks=62, unavailable=61, "
                "occluded=60, surface_faults=1, pre_acquire_epoch_changes=1, "
                "drawable_acquire_attempts=60, device_losses=0, configuration_error=false, "
                "dropped_views=60, dropped_stale_views=0, dropped_surface=60\n"
            )
            log.write_text(summary)
            result = CompletedProcess([], 1)
            occluded = dict(status="Occluded", scale=2, presented=False, drawable=None, transition_error=None)
            initial = dict(occluded, sequence=1, window=[1920, 1080])
            resized = dict(occluded, sequence=50, window=[1552, 896])
            evidence = occluded_transition(result, arm, [initial, resized])
            self.assertEqual(evidence["result"], "geometry_only")
            self.assertEqual(evidence["surface"]["pre_acquire_epoch_changes"], 1)
            self.assertEqual(evidence["surface"]["drawable_acquire_attempts"], 60)
            for changed in (dict(window=[1904, 1072]), dict(drawable=[[1552, 896], [1552, 896]]),
                            dict(presented=True), dict(status="DrawableTimeout"),
                            dict(status="NeedsReconfigure"), dict(transition_error="configuration refused")):
                with self.subTest(changed=changed), self.assertRaises(ValueError):
                    occluded_transition(result, arm, [initial, resized | changed])
            for field, change in (("occluded=60", "occluded=0"),
                                  ("pre_acquire_epoch_changes=1", "pre_acquire_epoch_changes=0"),
                                  ("drawable_acquire_attempts=60", "drawable_acquire_attempts=61"),
                                  ("device_losses=0", "device_losses=1"),
                                  ("configuration_error=false", "configuration_error=true")):
                log.write_text(summary.replace(field, change))
                with self.subTest(field=field), self.assertRaises(ValueError):
                    occluded_transition(result, arm, [initial, resized])


if __name__ == "__main__":
    unittest.main()
