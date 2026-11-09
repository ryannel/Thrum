"""Capture completeness and deferred lossless packaging refusal controls."""
import copy
import hashlib
from pathlib import Path
import tempfile
import unittest

from capture_v1_runtime import validate_runtime
from t2_writer_probe import FRAME_BYTES, package


def runtime():
    return {"complete": True, "backend": "Metal", "scenario": "pan",
            "opportunities": [{"submitted": True, "submission_id": 4, "accepted_ns": 9}],
            "images": [{"submission_id": 4, "submit_ns": 10, "map_observed_ns": 12}]}


class CaptureTests(unittest.TestCase):
    def test_complete_requires_exact_submission_image_and_raw_byte_coverage(self):
        self.assertEqual(validate_runtime(runtime(), FRAME_BYTES), [{"submission_id": 4, "offset": 0}])
        for change in (lambda r: r.update(complete=False),
                       lambda r: r.update(backend="software"),
                       lambda r: r["images"].clear(),
                       lambda r: r["images"].append(copy.deepcopy(r["images"][0])),
                       lambda r: r["images"][0].update(submission_id=5),
                       lambda r: r["images"][0].update(submit_ns=8),
                       lambda r: r["images"][0].update(map_observed_ns=9)):
            record = runtime()
            change(record)
            with self.assertRaises(ValueError):
                validate_runtime(record, FRAME_BYTES)
        with self.assertRaises(ValueError):
            validate_runtime(runtime(), FRAME_BYTES - 1)

    def test_capture_off_cannot_sneak_in_pixels(self):
        record = runtime()
        record.update(scenario="off", images=[])
        self.assertEqual(validate_runtime(record, 0), [])
        with self.assertRaises(ValueError):
            validate_runtime(record, FRAME_BYTES)

    def test_png_round_trip_checks_full_resolution_rgba_bytes(self):
        raw = bytes(range(256)) * (FRAME_BYTES // 256)
        with tempfile.TemporaryDirectory() as directory:
            self.assertEqual(package(raw, Path(directory) / "source.png"), hashlib.sha256(raw).hexdigest())
            with self.assertRaises(ValueError):
                package(raw[:-7680], Path(directory) / "short.png")

    def test_corrupt_packaged_output_is_refused(self):
        class CorruptOutput:
            def write_bytes(self, pixels):
                self.pixels = bytearray(pixels)
                self.pixels[20] ^= 1

            def read_bytes(self):
                return self.pixels

        with self.assertRaisesRegex(ValueError, "CRC"):
            package(bytes(FRAME_BYTES), CorruptOutput())


if __name__ == "__main__":
    unittest.main()
