"""Capture completeness and deferred lossless packaging refusal controls."""
import copy
import hashlib
import json
import shutil
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from capture_v1_simulator import export_session
from capture_v1_runtime import validate_runtime
from compare_v1_pixels import compare, load_pixels
from cooked_draw_ios_sim import console_witness
from cooked_draw_sim_launcher import validate_preflights
from t2_writer_probe import FRAME_BYTES, package


class SimulatorConsoleSetup(unittest.TestCase):
    def test_locked_unlocked_and_changed_console_states_are_observations(self):
        with patch('cooked_draw_ios_sim.time.sleep') as sleep:
            self.assertEqual(console_witness('"IOConsoleLocked" = Yes'), {'locked': True})
            self.assertEqual(console_witness('"IOConsoleLocked" = No'), {'locked': False})
            observations = [console_witness(raw)['locked'] for raw in [
                '"IOConsoleLocked" = No', '"IOConsoleLocked" = Yes', '"IOConsoleLocked" = No']]
            self.assertEqual(observations, [False, True, False])
            sleep.assert_not_called()

    def test_unavailable_console_state_retains_a_reason(self):
        self.assertEqual(console_witness('', 'ioreg failed'), {'locked': None, 'reason': 'ioreg failed'})
        self.assertEqual(console_witness('unrelated properties')['locked'], None)
        self.assertIn('unavailable', console_witness('unrelated properties')['reason'])

    def test_denied_preflights_require_the_actual_route_launcher(self):
        launcher = {'screen_recording_before': False, 'screen_recording_after': False,
                    'driver_exit_code': 0, 'bundle_identifier': 'qualification-app',
                    'launcher_pid': 101, 'driver_pid': 102, 'launcher_executable': '/qualification/Launcher',
                    'preflight_api': 'CGPreflightScreenCaptureAccess'}
        route = {'launcher_pid': 101, 'harness_pid': 102, 'route_pass': True}
        validate_preflights(launcher, route, 'qualification-app')
        for before, after in [(True, True), (False, False), (True, False), (False, True), (None, None)]:
            observed = dict(route, console_locked_before=before, console_locked_after=after)
            validate_preflights(launcher, observed, 'qualification-app')
        for field in ['screen_recording_before', 'screen_recording_after']:
            for value in [None, True, 'false', 0]:
                with self.subTest(field=field, value=value):
                    invalid = dict(launcher, **{field: value})
                    with self.assertRaises(RuntimeError):
                        validate_preflights(invalid, route, 'qualification-app')
            invalid = dict(launcher)
            invalid.pop(field)
            with self.assertRaises(RuntimeError):
                validate_preflights(invalid, route, 'qualification-app')
        for change in [{'error': 'preflight failed'}, {'driver_exit_code': 1},
                       {'bundle_identifier': 'unrelated-app'}, {'launcher_pid': 999},
                       {'driver_pid': 999}, {'preflight_api': ''}, {'launcher_executable': ''}]:
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                validate_preflights(dict(launcher, **change), route, 'qualification-app')
        with self.assertRaises(RuntimeError):
            validate_preflights(launcher, dict(route, route_pass=False), 'qualification-app')


class SimulatorArtifactExport(unittest.TestCase):
    def test_unexpected_control_completion_retains_diagnostics_before_refusal(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'source'
            source.mkdir()
            record = {'complete': True, 'images': []}
            (source / 'runtime.json').write_text(json.dumps(record))
            (source / 'frames.spool').write_bytes(b'original source')
            with self.assertRaisesRegex(ValueError, 'incorrectly completed'):
                export_session(source, root / 'export', negative=True)
            self.assertEqual(json.loads((root / 'export/runtime.json').read_text()), record)
            self.assertEqual((root / 'export/frames.spool').read_bytes(), b'original source')

    def test_failed_spool_copy_keeps_compact_runtime_diagnostics(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'source'
            source.mkdir()
            (source / 'runtime.json').write_text('{"complete":false}')
            (source / 'frames.spool').write_bytes(b'original source')
            copyfile = shutil.copyfile
            def failing_spool_copy(src, dest, **kwargs):
                if Path(src).name == 'frames.spool':
                    raise OSError('disk full')
                return copyfile(src, dest, **kwargs)
            with patch('capture_v1_simulator.shutil.copyfile', side_effect=failing_spool_copy):
                with self.assertRaisesRegex(OSError, 'disk full'):
                    export_session(source, root / 'export')
            self.assertEqual(json.loads((root / 'export/runtime.json').read_text()), {'complete': False})


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
class PixelComparison(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def raw(self, name, data):
        path = self.root / name
        path.write_bytes(data)
        return path

    def test_one_alpha_byte_fails_and_masks_retain_native_coordinates(self):
        baseline = bytes([15, 16, 17, 255] * 12)
        changed = bytearray(baseline)
        changed[(2 * 4 + 3) * 4 + 3] = 254
        left = self.raw('left.rgba', baseline)
        right = self.raw('right.rgba', changed)
        output = self.root / 'diff'
        result = compare(left, right, 'rgba8', [4, 3], output)
        self.assertFalse(result['equal'])
        self.assertEqual(result['changed_pixels'], 1)
        self.assertEqual(result['changed_bytes'], 1)
        self.assertEqual(result['bounds_xyxy_exclusive'], [3, 2, 4, 3])
        identity, crop = load_pixels(output / 'difference-crop.png', 'png', None)
        self.assertEqual(identity['extent'], [1, 1])
        self.assertEqual(crop, b'\xff' * 4)
        _, mask = load_pixels(output / 'difference.png', 'png', None)
        self.assertEqual(mask[-4:], b'\xff' * 4)
        self.assertEqual(mask[:-4], b'\0\0\0\xff' * 11)

    def test_half_float_signed_zero_is_distinct_without_float_conversion(self):
        left = self.raw('positive.rgba16f', bytes(8))
        right = self.raw('negative.rgba16f', bytes(7) + b'\x80')
        result = compare(left, right, 'rgba16f', [1, 1], self.root / 'diff')
        self.assertFalse(result['equal'])
        self.assertEqual(result['changed_bytes'], 1)

    def test_equal_pixels_have_hashes_and_no_difference_artifacts(self):
        raw = bytes(range(16))
        left = self.root / 'left.png'
        right = self.root / 'right.png'
        package(raw, left, 2, 2)
        package(raw, right, 2, 2)
        output = self.root / 'same'
        result = compare(left, right, 'png', None, output)
        self.assertTrue(result['equal'])
        self.assertEqual(result['reference']['pixels_sha256'], hashlib.sha256(raw).hexdigest())
        self.assertEqual(sorted(p.name for p in output.iterdir()), ['comparison.json'])
        self.assertEqual(json.loads((output / 'comparison.json').read_text()), result)
        with self.assertRaisesRegex(ValueError, 'new directory'):
            compare(left, right, 'png', None, output)

    def test_raw_requires_extent_and_complete_pixels(self):
        path = self.raw('partial', bytes(7))
        with self.assertRaisesRegex(ValueError, 'require --extent'):
            load_pixels(path, 'rgba16f', None)
        with self.assertRaisesRegex(ValueError, 'image length'):
            load_pixels(path, 'rgba16f', [1, 1])
        with self.assertRaisesRegex(ValueError, 'excessive'):
            load_pixels(path, 'rgba8', [-1, 1])

    def test_extent_and_channel_mismatch_are_refused(self):
        a, b = self.root / 'a.png', self.root / 'b.png'
        package(bytes(16), a, 2, 2)
        package(bytes(16), b, 1, 4)
        with self.assertRaisesRegex(ValueError, 'layouts differ'):
            compare(a, b, 'png', None, self.root / 'diff')
        with self.assertRaisesRegex(ValueError, 'differs from --extent'):
            load_pixels(a, 'png', [1, 4])
        ppm = self.raw('rgb.ppm', b'P6\n1 1\n255\n\0\0\0')
        identity, pixels = load_pixels(ppm, 'ppm', None)
        self.assertEqual(identity['bytes_per_pixel'], 3)
        self.assertEqual(pixels, bytes(3))

    def test_corrupted_png_crc_is_refused_before_comparison(self):
        path = self.root / 'bad.png'
        package(bytes(16), path, 2, 2)
        raw = bytearray(path.read_bytes())
        raw[-1] ^= 1
        path.write_bytes(raw)
        with self.assertRaisesRegex(ValueError, 'CRC mismatch'):
            load_pixels(path, 'png', None)
