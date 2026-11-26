"""Known answers for the exit frame reporter's deadline and callback accounting."""

import unittest

from showcase_frames import chrome_callback_misses, mac_opportunities


class ShowcaseFrameTests(unittest.TestCase):
    def test_gap_spans_four_periods_and_work_can_start_several_callbacks_earlier(self):
        callbacks = [{'target': index / 60, 'media': index / 60 - 0.005, 'sequence': index}
                     for index in range(1, 10)]
        presents = [
            {'present_media': time, 'submission': submission, 'sequence': sequence, 'world_tick': tick, 'work': []}
            for time, submission, sequence, tick in [
                (0.025, 1, 1, 30), (0.041, 2, 2, 31), (0.108, 3, 3, 35), (0.124, 4, 7, 36),
            ]
        ]
        presents[2]['work'] = [('submit and present', 0.042, 0.110)]
        _, misses = mac_opportunities(presents, callbacks, presents, {}, 31, 35, False)
        self.assertEqual(len(misses), 3)
        for miss in misses:
            self.assertAlmostEqual(miss['duration_ms'], 67)
            self.assertAlmostEqual(miss['opportunity_ms'], 1000 / 60)
            self.assertEqual((miss['previous_submission'], miss['next_submission']), (2, 3))
            self.assertEqual(miss['work_crossing_deadline'][0]['sequence'], 3)

    def test_chrome_pairs_before_clipping_and_distinguishes_work_from_waiting(self):
        callbacks = [
            {'tick': tick, 'raf_ms': raf, 'requested_ms': raf + 1,
             'returned_ms': raf + 2, 'drain_finished_ms': finish}
            for tick, raf, finish in [(29, 0, 3), (31, 50, 53), (40, 100, 179), (44, 183, 186), (48, 233, 236)]
        ]
        entry, inside, exit_ = chrome_callback_misses(callbacks, [], 31, 42, True)
        self.assertEqual([miss['position']['world_tick'] for miss in (entry, inside, exit_)], [31, 40, 42])
        self.assertEqual(exit_['position']['solar_hours'], 24)
        self.assertEqual((entry['duration_ms'], exit_['duration_ms']), (50, 83))
        self.assertEqual(entry['investigation'], 'unresolved under load')
        self.assertEqual(inside['investigation'], 'unresolved under load')
        self.assertEqual(exit_['page_work_past_callback_budget'][0]['stage'], 'page work after frame return')
        self.assertGreater(exit_['page_work_past_callback_budget'][0]['past_callback_budget_ms'], 60)

    def test_chrome_retains_partial_work_without_attributing_the_remaining_gap(self):
        callbacks = [
            {'tick': 31, 'raf_ms': 0, 'requested_ms': 1, 'returned_ms': 20, 'drain_finished_ms': 21},
            {'tick': 38, 'raf_ms': 100, 'requested_ms': 101, 'returned_ms': 102, 'drain_finished_ms': 103},
        ]
        [miss] = chrome_callback_misses(callbacks, [], 31, 42, False)
        self.assertTrue(miss['page_work_past_callback_budget'])
        self.assertEqual(miss['remaining_wait_ms'], 80)
        self.assertEqual(miss['investigation'], 'unresolved under load')


if __name__ == '__main__':
    unittest.main()
