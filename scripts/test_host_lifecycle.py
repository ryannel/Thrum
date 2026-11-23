"""Simulator launch ownership for the direct command and milestone runner."""

import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import host_lifecycle


class SimulatorDispatch(unittest.TestCase):
    def dispatch(self, inherited):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            output = root / 'tmp' / 'run'
            argv = ['host_lifecycle.py', 'check', '--target', 'ios-sim', '--output', str(output)]
            with patch.object(host_lifecycle, 'ROOT', root), patch.object(sys, 'argv', argv), \
                 patch.object(host_lifecycle, 'inherits_lock', return_value=inherited), \
                 patch.object(host_lifecycle, 'check_ios') as direct, \
                 patch.object(host_lifecycle, 'check_locked') as locked, \
                 patch.object(host_lifecycle.subprocess, 'check_output', return_value='revision\n'):
                host_lifecycle.main()
            if inherited:
                direct.assert_called_once_with(output / 'ios')
                locked.assert_not_called()
            else:
                locked.assert_called_once_with('simulator', 'ios-sim', output / 'ios')
                direct.assert_not_called()

    def test_unlocked_command_queues_for_simulator(self):
        self.dispatch(inherited=False)

    def test_coordinator_reuses_its_simulator_lock(self):
        self.dispatch(inherited=True)

    def test_only_a_holder_in_the_process_ancestry_owns_this_launch(self):
        processes = '81 80\n80 79\n79 1\n90 1\n'
        for holder, expected in [(79, True), (90, False)]:
            with self.subTest(holder=holder):
                report = json.dumps({'locks': [{'lock': 'simulator', 'holders': [{'pid': holder}]}]})
                with patch.dict(host_lifecycle.os.environ, {'ORCHESTRATE_SCRIPT': 'agent_loop.py'}), \
                     patch.object(host_lifecycle.os, 'getppid', return_value=81), \
                     patch.object(host_lifecycle.subprocess, 'check_output', side_effect=[report, processes]):
                    self.assertEqual(host_lifecycle.inherits_lock('simulator'), expected)


if __name__ == '__main__':
    unittest.main()
