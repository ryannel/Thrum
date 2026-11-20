"""The sprite cost verifier requires provenance and reports timing tripwires."""

from contextlib import redirect_stdout
import gzip
from io import StringIO
import json
import unittest

from render2d import FIXTURES, TARGETS, validate


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


if __name__ == "__main__":
    unittest.main()
