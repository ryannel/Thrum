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
        with self.assertRaisesRegex(cadence.IncompleteWitness, 'unmatched drawable 4'):
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
                   'place': 'encode_submit', 'request_place': 'encode_submit'}
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
                patch.object(cadence, 'table', side_effect=lambda path: (
                    [{'submission_id': '11', 'timing_requested': 'false'}]
                    if path.name == 'cadence-frames.tsv' else []
                )),
            ):
                for call, dispatch, opportunity, place, source in [
                    ('-0.001', '-0.003', 'late_request', 'after_present_call', request),
                    ('0.001', '-0.003', 'late_request', 'encode_submit', request),
                    ('0.001', '0.0005', 'display_link', 'display_link', missed),
                ]:
                    with self.subTest(call=call, opportunity=opportunity):
                        request['present_call_media_seconds'] = call
                        frame['opportunity_media_seconds'] = dispatch
                        missed['place'] = opportunity
                        (arm / 'witness.json').write_text(json.dumps(witness))
                        run = cadence.analyze_arm(arm, cadence.ABSENT)
                        (stall,) = run['stalls']
                        (loss,) = stall['refreshes']
                        self.assertEqual(loss['place'], place)
                        self.assertEqual(loss['account'], source)
                        self.assertEqual(loss['next_present_call_media_seconds'], call)

    def test_only_an_elapsed_refusal_can_explain_a_loss_before_the_next_dispatch(self):
        target = Decimal(-1)
        following = {'drawable': 12, 'opportunity_media_seconds': '0.001'}
        refused = {'submission_id': '', 'place': 'frame_slot'}
        future = {'submission_id': '12', 'place': 'drawable', 'request_place': 'drawable'}
        rows = {target: refused, Decimal(1): future}
        arm = Path('04-off')
        self.assertEqual(cadence.before_present_call(
            arm, Decimal(0), following, future, rows, sorted(rows), {target: Decimal(-2)}),
            ('frame_slot', refused, 'unserved_opportunity'))
        with self.assertRaisesRegex(InvalidMeasurement, 'no causal refresh-account row'):
            cadence.before_present_call(
                arm, Decimal(0), following, future, rows, sorted(rows), {target: Decimal(1)})


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
    def test_failed_arm_retries_only_with_recorded_occlusion(self):
        with TemporaryDirectory() as folder:
            arm = Path(folder) / '00-off'
            arm.mkdir()
            with self.assertRaisesRegex(InvalidMeasurement, 'exit 54'):
                capture.require_finished_arm(arm, 54)
            conditions = arm / 'refresh-window.txt'
            conditions.write_text('occluded_window_events=0\noccluded_surface_attempts=0\n')
            with self.assertRaisesRegex(InvalidMeasurement, 'exit 54'):
                capture.require_finished_arm(arm, 54)
            conditions.write_text('occluded_window_events=1\noccluded_surface_attempts=2400\n')
            with self.assertRaisesRegex(OccludedMeasurement, '2400 surface attempts'):
                capture.require_finished_arm(arm, 54)

    def test_prepare_cycles_display_only_when_first_wake_leaves_flag_set(self):
        with (
            patch.object(capture, 'console_unlocked', side_effect=[False, True]),
            patch.object(capture.subprocess, 'run') as run,
            patch.object(capture.time, 'sleep') as sleep,
        ):
            capture.require_unlocked_panel(wake=True)
        self.assertEqual([call.args[0][0] for call in run.call_args_list],
                         ['/usr/bin/caffeinate', '/usr/bin/pmset', '/usr/bin/caffeinate'])
        sleep.assert_called_once_with(3)

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

    def test_cadence_restarts_after_a_recorder_timeout(self):
        with (
            patch.object(capture, 'series', side_effect=[
                capture.RecordingTimeout('02-off'), {'cloud_runs': []},
            ]) as collect,
            redirect_stderr(StringIO()),
            redirect_stdout(StringIO()),
        ):
            self.assertEqual(capture.run_wind('cadence', None), 0)
        first, second = collect.call_args_list
        self.assertNotEqual(first.args[0], second.args[0])
        self.assertFalse(second.kwargs['resume'])

    def test_cadence_restarts_after_an_unmatched_observation(self):
        with (
            patch.object(capture, 'series', side_effect=[
                cadence.IncompleteWitness('unmatched drawable 1473'), {'cloud_runs': []},
            ]) as collect,
            redirect_stderr(StringIO()),
            redirect_stdout(StringIO()),
        ):
            self.assertEqual(capture.run_wind('cadence', None), 0)
        first, second = collect.call_args_list
        self.assertNotEqual(first.args[0], second.args[0])
        self.assertFalse(second.kwargs['resume'])


if __name__ == '__main__':
    unittest.main()
