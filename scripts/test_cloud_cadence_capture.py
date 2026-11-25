"""Check that occlusion recovery never combines arms from different series."""
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

from cloud_cadence import InvalidMeasurement, OccludedMeasurement
import cloud_cadence_capture as capture


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
