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



class SpoolTests(unittest.TestCase):
    def test_independent_decoder_checks_original_hash_truncation_and_extra_data(self):
        import subprocess
        from unittest.mock import patch
        import capture_spool
        originals = [bytes(range(17)), bytes(reversed(range(17)))]
        deltas = [originals[0], bytes(a ^ b for a, b in zip(*originals))]
        expected = [hashlib.sha256(data).hexdigest() for data in originals]
        with tempfile.TemporaryDirectory() as directory, patch.object(capture_spool, 'FRAME_BYTES', 17):
            path = Path(directory) / 'source.zst'
            encoded = subprocess.check_output(['zstd', '-q', '-c'], input=b''.join(deltas))
            path.write_bytes(encoded)
            self.assertEqual(list(capture_spool.decoded_frames(path, 'ZstdDelta', expected)), originals)
            with self.assertRaises(ValueError):
                list(capture_spool.decoded_frames(path, 'ZstdDelta', ['0' * 64] * 2))
            path.write_bytes(encoded[:-3])
            with self.assertRaises(ValueError):
                list(capture_spool.decoded_frames(path, 'ZstdDelta', expected))
            path.write_bytes(encoded)
            with self.assertRaises(ValueError):
                list(capture_spool.decoded_frames(path, 'ZstdDelta', expected[:1]))

if __name__ == "__main__":
    unittest.main()
