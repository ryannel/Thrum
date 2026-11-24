"""Simulator launch ownership for the direct command and milestone runner."""

import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import host_lifecycle
from host_lifecycle import simulator


class SimulatorDispatch(unittest.TestCase):
    def dispatch(self, inherited):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            output = root / 'tmp' / 'run'
            argv = ['host_lifecycle.py', 'check', '--target', 'ios-sim', '--output', str(output)]
            with patch.object(host_lifecycle, 'ROOT', root), patch.object(sys, 'argv', argv), \
                 patch.object(simulator, 'admission', return_value=({'pid': 79} if inherited else None, 1800)), \
                 patch.object(host_lifecycle, 'check_ios') as direct, \
                 patch.object(simulator, 'exec_with_admission') as locked, \
                 patch.object(host_lifecycle.v1_simulator_route, 'prepare') as prepare, \
                 patch.object(host_lifecycle, 'environment', return_value={}), \
                 patch.object(host_lifecycle.subprocess, 'check_output', return_value='revision\n'):
                host_lifecycle.main()
            if inherited:
                direct.assert_called_once_with(output / 'ios')
                locked.assert_not_called()
                prepare.assert_not_called()
            else:
                locked.assert_called_once_with([sys.executable, host_lifecycle.__file__, 'run-target',
                                                '--target', 'ios-sim', '--output', output / 'ios',
                                                '--ios-build-prepared'])
                prepare.assert_called_once_with({'CARGO_TARGET_DIR': str(root / 'target')})
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
                    admitted, _ = simulator.admission()
                    self.assertEqual(bool(admitted), expected)


if __name__ == '__main__':
    unittest.main()
