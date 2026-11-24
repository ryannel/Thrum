"""CPU tests for simulator ownership, bounded workers and verified cleanup."""
import json
import signal
import subprocess
import sys
import unittest
from unittest.mock import Mock, patch

import simulator_lifecycle as simulator


class SimulatedDevice:
    def __init__(self, state="Shutdown"):
        self.state = state
        self.other = "Booted"
        self.commands = []
        self.events = []
        self.shutdown_fails = False

    def invoke(self, argv, **_kwargs):
        command, *args = argv[2:]
        self.commands.append((command, *args))
        output = ""
        if command == "list":
            output = json.dumps({"devices": {simulator.RUNTIME: [
                {"name": simulator.NAME, "udid": "owned", "state": self.state,
                 "deviceTypeIdentifier": simulator.DEVICE_TYPE},
                {"name": "iPhone 18 Pro", "udid": "unrelated", "state": self.other,
                 "deviceTypeIdentifier": simulator.DEVICE_TYPE},
            ]}})
        elif command == "bootstatus":
            assert self.state == "Shutdown"
            self.state = "Booted"
        elif command == "shutdown" and not self.shutdown_fails:
            assert args == ["owned"]
            self.state = "Shutdown"
        return subprocess.CompletedProcess(argv, 0, output, "")

    def run(self, turn, argv, **kwargs):
        turn.check()
        return self.invoke(list(argv), **kwargs)


class SimulatorLifecycle(unittest.TestCase):
    def setUp(self):
        self.device = SimulatedDevice()
        self.holder = {"pid": 55, "held_s": 100}
        self.now = 10
        for target, replacement in (
            ("ancestor_locks", lambda: ({"simulator": self.holder}, 1800)),
            ("subprocess.run", self.device.invoke),
            ("Turn.run", lambda turn, argv, **kwargs: self.device.run(turn, argv, **kwargs)),
            ("Turn.request_shutdown", lambda turn: self.shutdown_request(turn)),
            ("Turn.event", lambda _, name, **fields: self.device.events.append((name, fields))),
        ):
            mock = patch(f"simulator_lifecycle.{target}", replacement)
            mock.start()
            self.addCleanup(mock.stop)

    def shutdown_request(self, turn):
        self.device.invoke(["xcrun", "simctl", "shutdown", turn.udid])
        return Mock(poll=lambda: 0)

    def turn(self, **kwargs):
        return simulator.supervise(clock=lambda: self.now, **kwargs)

    def test_two_operations_share_one_admission_and_separate_cases_end_shut(self):
        for _ in range(2):
            with self.turn() as first:
                self.assertEqual(first.holder, self.holder)
                for command in ("first", "second"):
                    first.simctl("spawn", first.udid, command)
            self.assertEqual(self.device.state, "Shutdown")
        self.assertEqual(self.device.commands.count(("bootstatus", "owned", "-b")), 2)
        self.assertEqual(self.device.commands.count(("spawn", "owned", "first")), 2)
        self.assertEqual(self.device.commands.count(("spawn", "owned", "second")), 2)
        self.assertEqual([name for name, _ in self.device.events],
                         ["admitted", "booted", "shutdown_verified"] * 2)
        self.assertEqual(self.device.other, "Booted")

    def test_gpu_ancestor_refused_before_device_use(self):
        with patch.object(simulator, "ancestor_locks", return_value=({"gpu": {}}, 1800)):
            with self.assertRaisesRegex(RuntimeError, "GPU lock"):
                with self.turn():
                    self.fail("device admitted from GPU lock")
        self.assertEqual(self.device.commands, [])

    def test_worker_failure_verifies_shutdown(self):
        with self.assertRaisesRegex(ValueError, "worker failure"):
            with self.turn():
                raise ValueError("worker failure")
        self.assertEqual(self.device.state, "Shutdown")
        self.assertEqual(self.device.events[-1][0], "shutdown_verified")

    def test_deadline_subtracts_admission_time_and_reserves_cleanup(self):
        with self.assertRaises(TimeoutError):
            with self.turn() as turn:
                self.assertEqual(turn.deadline, 1683)
                self.now = turn.deadline
                turn.check()
        self.assertEqual(self.device.state, "Shutdown")
        self.assertEqual([name for name, _ in self.device.events][-2:],
                         ["worker_deadline", "shutdown_verified"])

    def test_cancellation_remains_visible(self):
        with self.assertRaises(SystemExit) as result:
            with self.turn() as turn:
                turn.signal(signal.SIGTERM, None)
        self.assertEqual(result.exception.code, 143)
        self.assertEqual(self.device.state, "Shutdown")
        self.assertEqual([name for name, _ in self.device.events][-2:],
                         ["worker_cancelled", "shutdown_verified"])

    def test_failure_reports_state_and_next_holder_recovers_only_owned(self):
        self.device.shutdown_fails = True
        with patch.object(simulator, "CLEANUP_SECONDS", 0.01):
            with self.assertRaisesRegex(RuntimeError, "owned.*Booted"):
                with self.turn():
                    pass
        self.assertEqual(self.device.state, "Booted")
        self.device.shutdown_fails = False
        with self.turn(fresh=True):
            pass
        self.assertEqual(self.device.state, "Shutdown")
        self.assertEqual(self.device.other, "Booted")
        names = [name for name, _ in self.device.events]
        recovered = names.index("recover_stranded")
        self.assertEqual(names[recovered:recovered + 4],
                         ["recover_stranded", "shutdown_verified", "booted", "shutdown_verified"])
        self.assertIn(("erase", "owned"), self.device.commands)

    def test_refuses_caller_udid_outside_ownership(self):
        with self.assertRaisesRegex(RuntimeError, "outside this proof"):
            with self.turn(requested="unrelated"):
                self.fail("unrelated device admitted")
        self.assertEqual(self.device.commands, [("list", "devices", "-j")])


if __name__ == "__main__":
    unittest.main()
