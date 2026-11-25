"""Check that occlusion recovery never combines arms from different series."""
from contextlib import redirect_stderr, redirect_stdout
from decimal import Decimal
from io import StringIO
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

from cloud_cadence import InvalidMeasurement, OccludedMeasurement
import cloud_cadence as cadence
import cloud_cadence_capture as capture


class WindAttribution(unittest.TestCase):
    def test_witness_gap_requires_an_observation_inside_the_interval(self):
        witness = {
            'drawables': [
                {'drawable': 2, 'presented_media_ns': '-10'},
                {'drawable': 3, 'presented_media_ns': '0'},
                {'drawable': 5, 'presented_media_ns': '20'},
                {'drawable': 6, 'presented_media_ns': '30'},
            ],
            'unmatched': [{'drawable': 1}, {'drawable': 7}],
        }
        with self.assertRaisesRegex(InvalidMeasurement, 'unmatched drawable 4'):
            cadence.require_complete_observations(witness, Decimal(0), Decimal(20))
        witness['drawables'].insert(2, {'drawable': 4, 'presented_media_ns': '10'})
        cadence.require_complete_observations(witness, Decimal(0), Decimal(20))
        self.assertEqual(witness['unmatched'], [{'drawable': 1}, {'drawable': 7}])

    def test_next_drawables_call_decides_which_side_of_the_call_lost_the_refresh(self):
        frame = {
            'drawable': 11, 'presented_media_ns': str(cadence.PERIOD),
            'present_call_media_seconds': '0.002', 'opportunity_media_seconds': '-0.003',
        }
        witness = {
            'coverage': True, 'conversion_validated': True, 'reported_refresh_hz': 120, 'unmatched': [],
            'drawables': [
                {'drawable': 10, 'presented_media_ns': str(-cadence.PERIOD)}, frame,
            ],
        }
        request = {'target_media_seconds': '0.008', 'submission_id': '11',
                   'interval_submission_id': '', 'present_call_media_seconds': '-0.001',
                   'place': 'served'}
        missed = {'target_media_seconds': '0', 'submission_id': '', 'interval_submission_id': '',
                  'place': 'late_request'}
        rows = {Decimal(0): missed, Decimal(8_000_000): request}
        with TemporaryDirectory() as folder:
            arm = Path(folder)
            (arm / 'witness.json').write_text(json.dumps(witness))
            (arm / 'refresh-counts.txt').write_text('excess_present_records=0;')
            with (
                patch.object(cadence, 'window_conditions', return_value={
                    'occluded_window_events': '0', 'occluded_surface_attempts': '0',
                }),
                patch.object(cadence, 'interval', return_value=(Decimal(0), cadence.PERIOD)),
                patch.object(cadence, 'account_rows', return_value=rows),
                patch.object(cadence, 'table', return_value=[{
                    'submission_id': '11', 'timing_requested': 'false',
                }]),
            ):
                for call, place, source in [
                    ('-0.001', 'after_present_call', request), ('0.001', 'late_request', missed),
                ]:
                    with self.subTest(call=call):
                        request['present_call_media_seconds'] = call
                        run = cadence.analyze_arm(arm, cadence.ABSENT)
                        (stall,) = run['stalls']
                        (loss,) = stall['refreshes']
                        self.assertEqual(loss['place'], place)
                        self.assertEqual(loss['account'], source)
                        self.assertEqual(loss['next_present_call_media_seconds'], call)


class RecordingOwnership(unittest.TestCase):
    def test_uses_only_a_writable_descriptor_and_preserves_ambiguous_files(self):
        with TemporaryDirectory() as folder:
            temporary = Path(folder).resolve()
            owned, other = (temporary / f'instruments{name}.ktrace' for name in ('One', 'Two'))
            for path in (owned, other):
                path.write_text('unrelated recorder contents may include the same game')

            def fields(path, mode):
                info = path.stat()
                return (f'\nf4\0a{mode}\0tREG\0D{info.st_dev:#x}\0i{info.st_ino}\0n{path}\0')

            files = capture.writable_recordings(fields(owned, 'w') + fields(other, 'r'), temporary)
            self.assertEqual(capture.kernel_recording(files), owned)
            ambiguous = capture.writable_recordings(fields(owned, 'w') + fields(other, 'u'), temporary)
            with self.assertRaisesRegex(InvalidMeasurement, 'one owned kernel recording'):
                capture.kernel_recording(ambiguous)
            self.assertTrue(owned.is_file())
            self.assertTrue(other.is_file())
            moved = owned.with_suffix('.retained')
            owned.rename(moved)
            owned.symlink_to(other)
            with self.assertRaisesRegex(InvalidMeasurement, 'changed identity'):
                capture.kernel_recording(files)
            self.assertTrue(other.is_file())
            self.assertTrue(moved.is_file())


class WindRecovery(unittest.TestCase):
    def test_cadence_starts_a_fresh_series_after_occlusion(self):
        with TemporaryDirectory() as folder:
            working = Path(folder).resolve()
            log = StringIO()
            with (
                patch.object(capture, 'WORKING', working),
                patch.object(capture, 'series', side_effect=[
                    OccludedMeasurement('04-off'), {'cloud_runs': []},
                ]) as collect,
                redirect_stderr(log),
                redirect_stdout(log),
            ):
                self.assertEqual(capture.run_wind('cadence', None), 0)

            first, second = collect.call_args_list
            self.assertEqual(first.kwargs, {'diagnostics': True, 'resume': False})
            self.assertEqual(second.kwargs, first.kwargs)
            self.assertNotEqual(first.args[0], second.args[0])
            self.assertEqual(first.args[0].parent, working / 'WP19')
            self.assertEqual(second.args[0].parent, first.args[0].parent)
            self.assertIn(str(first.args[0] / 'series.json'), log.getvalue())
            self.assertIn(str(second.args[0] / 'series.json'), log.getvalue())

    def test_only_default_cadence_retries(self):
        cases = (
            ('cadence', None, False, InvalidMeasurement('witness coverage')),
            ('cadence', Path('provided'), False, OccludedMeasurement('00-off')),
            ('cadence', Path('provided'), True, OccludedMeasurement('04-off')),
            ('hold', None, False, OccludedMeasurement('00-off')),
        )
        for command, output, resume, error in cases:
            with self.subTest(command=command, output=output, resume=resume):
                with patch.object(capture, 'series', side_effect=error) as collect:
                    with self.assertRaises(type(error)):
                        capture.run_wind(command, output, resume=resume)
                collect.assert_called_once()

    def test_cadence_stops_after_three_occluded_series(self):
        with (
            patch.object(capture, 'series', side_effect=OccludedMeasurement('00-off')) as collect,
            redirect_stderr(StringIO()),
            self.assertRaises(OccludedMeasurement),
        ):
            capture.run_wind('cadence', None)
        self.assertEqual(collect.call_count, 3)
        self.assertEqual(len({call.args[0] for call in collect.call_args_list}), 3)


if __name__ == '__main__':
    unittest.main()
