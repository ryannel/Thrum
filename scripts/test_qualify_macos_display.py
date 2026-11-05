"""Checks for Xcode's displayed-surface export parser."""

import tempfile
import unittest
from pathlib import Path

from qualify_macos_display import displayed_swaps, summarize


class DisplayQualificationTests(unittest.TestCase):
    def test_process_and_display_filter_resolve_referenced_values(self):
        xml = """<trace-query-result><node>
          <row><start-time id="1">1000000000</start-time>
            <display-name id="2" fmt="Built-In Display 1"/>
            <narrative id="3" fmt="Surface 1 (v1-game (42):Frame 4:Surface 1)"/></row>
          <row><start-time id="4">1008333333</start-time>
            <display-name ref="2"/>
            <narrative id="5" fmt="Surface 2 (v1-game (42):Frame 5:Surface 2)"/></row>
          <row><start-time id="6">1016666666</start-time>
            <display-name ref="2"/>
            <narrative id="7" fmt="Surface 3 (other-v1-game (3):Frame 6:Surface 3)"/></row>
        </node></trace-query-result>"""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "export.xml"
            path.write_text(xml)
            swaps = displayed_swaps(path, "v1-game", "Built-In Display 1")
        self.assertEqual(swaps, [(1_000_000_000, 42, 4), (1_008_333_333, 42, 5)])

    def test_short_trace_and_real_missed_swaps_cannot_qualify(self):
        period = 8_333_333
        on_time = [(index * period, 42, index) for index in range(121)]
        short = summarize(on_time, 600.0)
        self.assertFalse(short["qualified"])
        self.assertEqual(short["intervals_over_1ms_error"], 0)

        missed = on_time[:]
        for index in range(80, len(missed)):
            missed[index] = (missed[index][0] + 2 * period, 42, missed[index][2])
        # One missed swap after index 79 must be reported even when too sparse
        # to move the p99 of a 120-interval run.
        result = summarize(missed, 0.1)
        self.assertEqual(result["intervals_over_1ms_error"], 1)

    def test_two_short_bursts_do_not_count_as_ten_minutes(self):
        period = 8_333_333
        first = [(index * period, 42, index) for index in range(121)]
        second = [
            (600_000_000_000 + index * period, 42, index + 121)
            for index in range(121)
        ]
        result = summarize(first + second, 600.0)
        self.assertGreater(result["observed_seconds"], 600)
        self.assertLess(result["estimated_refresh_coverage"], 0.01)
        self.assertFalse(result["qualified"])

    def test_two_processes_cannot_combine_into_one_120_hz_run(self):
        swaps = [
            (index * 8_333_333, 42 if index % 2 == 0 else 43, index)
            for index in range(121)
        ]
        with self.assertRaisesRegex(ValueError, "multiple V1 process IDs"):
            summarize(swaps, 0.1)


if __name__ == "__main__":
    unittest.main()
