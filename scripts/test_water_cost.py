"""Validate the three cache timing sources without opening a device."""
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

import water_cost


class SurfCacheComparison(unittest.TestCase):
    def test_requires_wp19_timings_in_both_phases(self):
        history = json.loads(water_cost.SURF_CACHE_INVESTIGATION.read_text())
        wp13 = json.loads(water_cost.WP13_SURF.read_text())
        # A minimal instrumented frame with all six counters.
        texels = dict(own_cell=56, swash_contour=0, across_cell_edge=0,
                      ring_edge=0, previous_stage=0, distant_lattice=56)
        nodes = dict(evaluated=1, allocated=2)
        stable = dict(mesh=nodes, distant_lattice=nodes, texels=texels)
        moved = dict(stable, texels=dict(texels, previous_stage=56))
        record = {key: history[key] for key in ('camera', 'extent', 'wind')}
        record['cache_work'] = dict(texels_per_full_cell_stage=56, texels_per_lag_cell_stage=48,
                                    stable_stage=stable, moved_stage=moved)
        table = {'surf': {phase: {'passes_ms': {'surf field cache': 200.0}}
                          for phase in ('step', 'repeat')}}

        with TemporaryDirectory() as directory:
            fixture = Path(directory) / 'history.json'
            fixture.write_text(json.dumps(history))
            with patch.object(water_cost, 'SURF_CACHE_INVESTIGATION', fixture):
                result = water_cost.surf_cache_investigation(record, table, wp13)
                for phase, value in history['wp19_review_fix']['cache_ms'].items():
                    self.assertEqual(result['cache_ms'][phase]['wp19_review_fix'], value)
                    self.assertEqual(result['cache_ms'][phase]['current'], 200.0)

                for phase in ('step', 'repeat'):
                    for invalid in ({}, {phase: None}, {phase: float('nan')}, {phase: True}):
                        with self.subTest(phase=phase, invalid=invalid):
                            times = dict(history['wp19_review_fix']['cache_ms'])
                            del times[phase]
                            times.update(invalid)
                            changed = dict(history, wp19_review_fix=dict(history['wp19_review_fix'],
                                                                        cache_ms=times))
                            fixture.write_text(json.dumps(changed))
                            with self.assertRaisesRegex(ValueError, f'WP19 review fix: .* {phase}'):
                                water_cost.surf_cache_investigation(record, table, wp13)


if __name__ == '__main__':
    unittest.main()
