"""Candidate native-build supervisor request and result checks."""

import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock

from run_bounded_native_build import observed_receipts, run


@unittest.skipUnless(sys.platform == "darwin", "first native supervisor profile is macOS")
class NativeBuildSupervisor(unittest.TestCase):
    @staticmethod
    def receipt(completed=True):
        def field(value):
            return len(value).to_bytes(4, "little") + value
        raw = (b"THRMRCP1" + (1).to_bytes(4, "little") + b"\0"
               + field(b"/tool/rustc") + field(b"/source")
               + (1).to_bytes(4, "little") + field(b"-vV")
               + (0).to_bytes(4, "little") + b"PEND")
        return raw + (b"DONE" + b"\0" + (0).to_bytes(4, "little")) if completed else raw

    def request(self, root):
        source = root / "source"
        source.mkdir()
        output = root / "output"
        output.mkdir()
        request = output / "cargo-supervisor-request.json"
        request.write_text(json.dumps({
            "format": 1,
            "command": ["/usr/bin/sandbox-exec", "-f", str(output / "build.sb"),
                        "/bin/true"],
            "cwd": str(source), "environment": {"CARGO_HOME": str(root / "cache")},
            "output": str(output), "ownerPid": os.getppid(),
        }))
        return request, output

    def test_completed_result_binds_bounded_output_and_parent(self):
        with tempfile.TemporaryDirectory() as directory:
            request, output = self.request(Path(directory))
            with mock.patch("run_bounded_native_build.collect_bounded",
                            return_value=(0, b'{"reason":"ok"}\n', b"")) as collector:
                self.assertEqual(run(request), 0)
            self.assertEqual(collector.call_args.kwargs["parent_pid"], os.getppid())
            self.assertEqual(collector.call_args.kwargs["stdout_limit"], 4 * 1024 * 1024)
            self.assertEqual((output / "cargo.stdout").read_bytes(), b'{"reason":"ok"}\n')
            report = json.loads((output / "cargo-supervisor.json").read_text())
            self.assertEqual(report["kind"], "completed")
            self.assertEqual(report["cargoStatus"], 0)
            self.assertEqual(report["stdoutSha256"], hashlib.sha256(
                b'{"reason":"ok"}\n').hexdigest())
            with self.assertRaisesRegex(ValueError, "already exists"):
                run(request)

    def test_wrong_owner_and_aliased_request_refuse_before_launch(self):
        with tempfile.TemporaryDirectory() as directory:
            request, _ = self.request(Path(directory))
            data = json.loads(request.read_text())
            data["ownerPid"] = os.getppid() + 1
            request.write_text(json.dumps(data))
            with mock.patch("run_bounded_native_build.collect_bounded") as collector:
                with self.assertRaisesRegex(ValueError, "lost its launching owner"):
                    run(request)
                collector.assert_not_called()
            target = request.with_suffix(".target")
            request.rename(target)
            request.symlink_to(target)
            with self.assertRaisesRegex(ValueError, "ordinary absolute file"):
                run(request)

    def test_signalled_or_unexpected_cargo_exit_is_refused(self):
        for cargo_status in (-9, 126):
            with self.subTest(cargo_status=cargo_status):
                with tempfile.TemporaryDirectory() as directory:
                    request, output = self.request(Path(directory))
                    with mock.patch("run_bounded_native_build.collect_bounded",
                                    return_value=(cargo_status, b"", b"")):
                        self.assertEqual(run(request), 125)
                    report = json.loads((output / "cargo-supervisor.json").read_text())
                    self.assertEqual(report["kind"], "refusedStatus")
                    self.assertEqual(report["cargoStatus"], cargo_status)
                    self.assertFalse((output / "cargo.stdout").exists())

    def test_interrupted_and_timed_out_jobs_inventory_bounded_receipts(self):
        for failure, expected_kind in ((KeyboardInterrupt("stopped"), "interrupted"),
                                       (ValueError("Cargo build timed out"), "supervisorError")):
            with self.subTest(expected_kind=expected_kind):
                with tempfile.TemporaryDirectory() as directory:
                    request, output = self.request(Path(directory))
                    receipts = output / "rustc-receipts"
                    receipts.mkdir()
                    (receipts / "101.receipt").write_bytes(self.receipt())
                    (receipts / "102.receipt").write_bytes(self.receipt(completed=False))
                    (receipts / "103.receipt").write_bytes(b"partial")
                    with mock.patch("run_bounded_native_build.collect_bounded",
                                    side_effect=failure):
                        self.assertEqual(run(request), 125)
                    report = json.loads((output / "cargo-supervisor.json").read_text())
                    self.assertEqual(report["kind"], expected_kind)
                    self.assertFalse(report["buildReady"])
                    self.assertEqual(report["receiptInventory"]["kind"], "observedOnly")
                    self.assertEqual(report["receiptInventory"]["files"], 3)
                    self.assertEqual(report["receiptInventory"]["completedSuccess"], 1)
                    self.assertEqual(report["receiptInventory"]["pending"], 1)
                    self.assertEqual(report["receiptInventory"]["unparsed"], 1)
                    self.assertEqual(len(report["receiptInventory"]["setSha256"]), 64)
                    self.assertFalse((output / "cargo.stdout").exists())

    def test_interrupted_receipt_alias_refuses_inventory_claim(self):
        with tempfile.TemporaryDirectory() as directory:
            request, output = self.request(Path(directory))
            receipts = output / "rustc-receipts"
            receipts.mkdir()
            outside = output / "outside"
            outside.write_bytes(self.receipt())
            (receipts / "101.receipt").symlink_to(outside)
            with self.assertRaisesRegex(ValueError, "aliased"):
                observed_receipts(output)
            with mock.patch("run_bounded_native_build.collect_bounded",
                            side_effect=ValueError("Cargo build timed out")):
                self.assertEqual(run(request), 125)
            report = json.loads((output / "cargo-supervisor.json").read_text())
            self.assertEqual(report["receiptInventory"]["kind"], "unavailable")
            self.assertFalse(report["buildReady"])

    def test_interrupted_receipt_inventory_refuses_65th_entry(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            receipts = output / "rustc-receipts"
            receipts.mkdir()
            for number in range(65):
                (receipts / f"{number}.receipt").write_bytes(b"partial")
            with self.assertRaisesRegex(ValueError, "exceeds limit"):
                observed_receipts(output)

    def test_sigterm_writes_interruption_report(self):
        with tempfile.TemporaryDirectory() as directory:
            request, output = self.request(Path(directory))
            data = json.loads(request.read_text())
            data["ownerPid"] = os.getpid()
            request.write_text(json.dumps(data))
            marker = output / "collector-started"
            child_code = (
                "import pathlib,sys,time;"
                "import run_bounded_native_build as native;"
                f"marker=pathlib.Path({str(marker)!r});"
                "def_collect=lambda *a,**k:(marker.write_text('ready'),time.sleep(10));"
                "native.collect_bounded=def_collect;"
                f"sys.exit(native.run(pathlib.Path({str(request)!r})))"
            )
            child = subprocess.Popen([sys.executable, "-c", child_code],
                                     cwd=Path(__file__).parent,
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            try:
                deadline = time.monotonic() + 3
                while not marker.exists() and time.monotonic() < deadline:
                    time.sleep(0.01)
                self.assertTrue(marker.exists(), "supervisor did not enter collector")
                os.kill(child.pid, signal.SIGTERM)
                child.communicate(timeout=4)
                self.assertEqual(child.returncode, 125)
                report = json.loads((output / "cargo-supervisor.json").read_text())
                self.assertEqual(report["kind"], "interrupted")
                self.assertFalse(report["buildReady"])
            finally:
                if child.poll() is None:
                    child.kill()
                    child.communicate()

    @unittest.skipUnless(os.environ.get("THRUM_TEST_NATIVE_SUPERVISOR_PROCESS") == "1",
                         "opt-in macOS sandbox process cleanup proof")
    def test_sigterm_and_timeout_stop_real_sandbox_child_before_fault_inventory(self):
        for fault, timeout in (("sigterm", 120), ("timeout", 2)):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as directory:
                request, output = self.request(Path(directory))
                receipt_dir = output / "rustc-receipts"
                receipt_dir.mkdir()
                pidfile = output / "cargo-child.pid"
                child_code = (
                    "import os,pathlib,time;"
                    f"pathlib.Path({str(receipt_dir / '101.receipt')!r}).write_bytes({self.receipt(completed=False)!r});"
                    f"marker=pathlib.Path({str(pidfile)!r});"
                    "temporary=marker.with_suffix('.tmp');"
                    "temporary.write_text(str(os.getpid()));temporary.replace(marker);"
                    "time.sleep(30)"
                )
                data = json.loads(request.read_text())
                data["ownerPid"] = os.getpid()
                data["command"] = ["/usr/bin/sandbox-exec", "-p", "(version 1)(allow default)",
                                   sys.executable, "-c", child_code]
                request.write_text(json.dumps(data))
                runner = (
                    "import pathlib,sys;import run_bounded_native_build as native;"
                    f"sys.exit(native.run(pathlib.Path({str(request)!r}),timeout_seconds={timeout}))"
                )
                supervisor = subprocess.Popen(
                    [sys.executable, "-c", runner], cwd=Path(__file__).parent,
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                )
                child_pid = None
                try:
                    deadline = time.monotonic() + 5
                    while not pidfile.exists() and time.monotonic() < deadline:
                        if supervisor.poll() is not None:
                            break
                        time.sleep(0.01)
                    self.assertTrue(pidfile.exists(), "sandbox child did not start")
                    child_pid = int(pidfile.read_text())
                    if fault == "sigterm":
                        os.kill(supervisor.pid, signal.SIGTERM)
                    _, stderr = supervisor.communicate(timeout=6)
                    self.assertEqual(supervisor.returncode, 125,
                                     stderr.decode(errors="replace"))
                    report = json.loads((output / "cargo-supervisor.json").read_text())
                    self.assertEqual(report["kind"],
                                     "interrupted" if fault == "sigterm" else "supervisorError")
                    self.assertFalse(report["buildReady"])
                    self.assertEqual(report["receiptInventory"]["pending"], 1)
                    self.assertEqual(report["receiptInventory"]["files"], 1)
                    deadline = time.monotonic() + 2
                    while time.monotonic() < deadline:
                        try:
                            os.kill(child_pid, 0)
                        except ProcessLookupError:
                            break
                        time.sleep(0.01)
                    else:
                        self.fail("same-group sandbox child survived supervisor fault")
                finally:
                    if supervisor.poll() is None:
                        supervisor.kill()
                        supervisor.communicate()
                    if child_pid is not None:
                        try:
                            os.kill(child_pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass


if __name__ == "__main__":
    unittest.main()
