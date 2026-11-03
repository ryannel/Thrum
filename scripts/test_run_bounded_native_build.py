"""Candidate native-build supervisor request and result checks."""

import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock

from run_bounded_native_build import observed_receipts, run
from verify_rustc_receipts import parse_receipt


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
                                    return_value=(cargo_status, b"partial output", b"sandbox refused")):
                        self.assertEqual(run(request), 125)
                    report = json.loads((output / "cargo-supervisor.json").read_text())
                    self.assertEqual(report["kind"], "refusedStatus")
                    self.assertEqual(report["cargoStatus"], cargo_status)
                    self.assertEqual((output / "cargo.stdout").read_bytes(), b"partial output")
                    self.assertEqual((output / "cargo.stderr").read_bytes(), b"sandbox refused")
                    self.assertEqual(report["stdoutSha256"], hashlib.sha256(b"partial output").hexdigest())
                    self.assertEqual(report["stderrSha256"], hashlib.sha256(b"sandbox refused").hexdigest())

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
        wrapper = Path(os.environ["THRUM_TEST_RUSTC_RECEIPT_WRAPPER"])
        self.assertTrue(wrapper.is_absolute() and wrapper.is_file() and not wrapper.is_symlink())
        selected_python = Path(sys.executable).resolve(strict=True)
        for fault, timeout in (("sigterm", 120), ("timeout", 2)):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as directory:
                request, output = self.request(Path(directory))
                receipt_dir = output / "rustc-receipts"
                receipt_dir.mkdir()
                pidfile = output / "cargo-child.pid"
                child_code = (
                    "import os,pathlib,time;"
                    f"marker=pathlib.Path({str(pidfile)!r});"
                    "temporary=marker.with_suffix('.tmp');"
                    "temporary.write_text(str(os.getpid()));temporary.replace(marker);"
                    "time.sleep(30)"
                )
                data = json.loads(request.read_text())
                data["ownerPid"] = os.getpid()
                data["environment"].update({
                    "THRUM_SELECTED_RUSTC": str(selected_python),
                    "THRUM_RECEIPT_DIR": str(receipt_dir),
                })
                data["command"] = ["/usr/bin/sandbox-exec", "-p", "(version 1)(allow default)",
                                   str(wrapper), str(selected_python), "-c", child_code]
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
                    receipt_paths = list(receipt_dir.iterdir())
                    self.assertEqual(len(receipt_paths), 1)
                    parsed = parse_receipt(receipt_paths[0].read_bytes(), allow_pending=True)
                    self.assertTrue(parsed["pending"])
                    self.assertEqual(parsed["rustc"], os.fsencode(selected_python))
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

    @unittest.skipUnless(os.environ.get("THRUM_TEST_NATIVE_SUPERVISOR_PROCESS") == "1",
                         "opt-in macOS Cargo interruption proof")
    def test_interrupted_cargo_compile_retains_pending_wrapper_receipt(self):
        self._cargo_compiler_fault("sigterm")

    @unittest.skipUnless(os.environ.get("THRUM_TEST_NATIVE_SUPERVISOR_PROCESS") == "1",
                         "opt-in macOS Cargo timeout proof")
    def test_timed_out_cargo_compile_retains_pending_wrapper_receipt(self):
        self._cargo_compiler_fault("timeout")

    @unittest.skipUnless(os.environ.get("THRUM_TEST_NATIVE_SUPERVISOR_PROCESS") == "1",
                         "opt-in macOS real Cargo receipt closure proof")
    def test_real_cargo_success_and_compile_failure_complete_receipts(self):
        wrapper = Path(os.environ["THRUM_TEST_RUSTC_RECEIPT_WRAPPER"])
        self.assertTrue(wrapper.is_absolute() and wrapper.is_file() and not wrapper.is_symlink())
        cargo = Path(subprocess.check_output(
            ["rustup", "which", "cargo", "--toolchain", "1.99.1"], text=True).strip())
        rustc = Path(subprocess.check_output(
            ["rustup", "which", "rustc", "--toolchain", "1.99.1"], text=True).strip())
        for broken, expected_status in ((False, 0), (True, 101)):
            with self.subTest(broken=broken):
                root = Path(tempfile.mkdtemp(prefix="thrum-cargo-receipt-"))
                job = {}
                try:
                    self._cargo_completed_case(root, job, wrapper, cargo, rustc,
                                               broken, expected_status)
                finally:
                    supervisor = job.get("supervisor")
                    status = None if supervisor is None else supervisor.poll()
                    report = root / "output/cargo-supervisor.json"
                    clean = supervisor is None or (status in (0, 101, 125) and report.is_file())
                    if supervisor is not None and status is None:
                        try:
                            supervisor.terminate()
                            supervisor.communicate(timeout=8)
                            clean = (supervisor.returncode in (0, 101, 125)
                                     and report.is_file())
                        except (OSError, subprocess.TimeoutExpired):
                            supervisor.kill()
                            try:
                                supervisor.communicate(timeout=2)
                            except subprocess.TimeoutExpired:
                                supervisor.stdout.close()
                                supervisor.stderr.close()
                                supervisor.wait(timeout=2)
                    if clean:
                        shutil.rmtree(root)
                    else:
                        self.fail(f"supervisor cleanup uncertain; inspect quarantined {root}")

    def _cargo_completed_case(self, root, job, wrapper, cargo, rustc,
                              broken, expected_status):
        request, output = self.request(root)
        source = root / "source"
        (source / "src").mkdir()
        (source / "Cargo.toml").write_text(
            '[package]\nname = "thrum_receipt_fixture"\nversion = "0.1.0"\n'
            'edition = "2024"\n')
        (source / "Cargo.lock").write_text(
            'version = 4\n\n[[package]]\nname = "thrum_receipt_fixture"\n'
            'version = "0.1.0"\n')
        (source / "src/main.rs").write_text(
            'fn main() { missing_name(); }\n' if broken else 'fn main() {}\n')
        receipt_dir = output / "rustc-receipts"
        receipt_dir.mkdir()
        cargo_home = root / "cargo-home"
        cargo_home.mkdir()
        data = json.loads(request.read_text())
        data["ownerPid"] = os.getpid()
        data["environment"] = {
            "CARGO_HOME": str(cargo_home),
            "CARGO_TARGET_DIR": str(output / "target"),
            "PATH": "/usr/bin:/bin",
            "RUSTC": str(rustc),
            "RUSTC_WRAPPER": str(wrapper),
            "THRUM_SELECTED_RUSTC": str(rustc),
            "THRUM_RECEIPT_DIR": str(receipt_dir),
        }
        data["command"] = [
            "/usr/bin/sandbox-exec", "-p", "(version 1)(allow default)(deny network*)",
            str(cargo), "build", "--locked", "--offline", "--manifest-path",
            str(source / "Cargo.toml"),
        ]
        request.write_text(json.dumps(data))
        runner = (
            "import pathlib,sys;import run_bounded_native_build as native;"
            f"sys.exit(native.run(pathlib.Path({str(request)!r}),timeout_seconds=10))"
        )
        supervisor = subprocess.Popen(
            [sys.executable, "-c", runner], cwd=Path(__file__).parent,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
        job["supervisor"] = supervisor
        _, stderr = supervisor.communicate(timeout=20)
        self.assertEqual(supervisor.returncode, expected_status,
                         stderr.decode(errors="replace"))
        report = json.loads((output / "cargo-supervisor.json").read_text())
        self.assertEqual(report["kind"], "completed")
        self.assertEqual(report["cargoStatus"], expected_status)
        self.assertFalse(report["buildReady"])
        for name in ("stdout", "stderr"):
            raw = (output / f"cargo.{name}").read_bytes()
            self.assertEqual(report[f"{name}Bytes"], len(raw))
            self.assertEqual(report[f"{name}Sha256"], hashlib.sha256(raw).hexdigest())
        receipts = [parse_receipt(path.read_bytes())
                    for path in sorted(receipt_dir.iterdir())]
        self.assertGreaterEqual(len(receipts), 2)
        self.assertTrue(all(item["rustc"] == os.fsencode(rustc)
                            and item["cwd"] == os.fsencode(source.resolve())
                            and b"THRUM_RECEIPT_DIR" not in item["environment"]
                            and b"THRUM_SELECTED_RUSTC" not in item["environment"]
                            for item in receipts),
                        [(item["kind"], item["rustc"], item["cwd"])
                         for item in receipts])
        compiles = [item for item in receipts if item["kind"] == 1]
        self.assertEqual(len(compiles), 1)
        self.assertEqual(compiles[0]["outcome"], 0)
        self.assertEqual(compiles[0]["status"], 1 if broken else 0)
        self.assertTrue(any(argument.endswith(b"src/main.rs")
                            for argument in compiles[0]["arguments"]))
        self.assertTrue(all(item["status"] == 0 for item in receipts
                            if item["kind"] == 0))
        self.assertEqual((output / "target/debug/thrum_receipt_fixture").exists(),
                         not broken)

    def _cargo_compiler_fault(self, fault):
        if fault not in ("sigterm", "timeout"):
            raise ValueError("unsupported Cargo compiler fault")
        wrapper = Path(os.environ["THRUM_TEST_RUSTC_RECEIPT_WRAPPER"])
        self.assertTrue(wrapper.is_absolute() and wrapper.is_file() and not wrapper.is_symlink())
        cargo = Path(subprocess.check_output(
            ["rustup", "which", "cargo", "--toolchain", "1.99.1"], text=True).strip())
        rustc = Path(subprocess.check_output(
            ["rustup", "which", "rustc", "--toolchain", "1.99.1"], text=True).strip())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            request, output = self.request(root)
            source = root / "source"
            (source / "src").mkdir()
            (source / "Cargo.toml").write_text(
                '[package]\nname = "thrum_interrupt_fixture"\nversion = "0.1.0"\n'
                'edition = "2024"\n')
            (source / "src/main.rs").write_text('fn main() {}\n')
            receipt_dir = output / "rustc-receipts"
            receipt_dir.mkdir()
            compile_pidfile = output / "compile.pid"
            compiler = output / "sleeping-rustc"
            compiler.write_text(
                f"#!{sys.executable}\n"
                "import os,pathlib,sys,time\n"
                f"if '--emit' in sys.argv or any(a.startswith('--emit=') for a in sys.argv):\n"
                f"    marker=pathlib.Path({str(compile_pidfile)!r})\n"
                "    temporary=marker.with_suffix('.tmp')\n"
                "    temporary.write_text(str(os.getpid()))\n"
                "    temporary.replace(marker)\n"
                "    time.sleep(30)\n"
                f"os.execv({str(rustc)!r}, [{str(rustc)!r}, *sys.argv[1:]])\n")
            compiler.chmod(0o700)
            data = json.loads(request.read_text())
            data["ownerPid"] = os.getpid()
            data["environment"] = {
                "CARGO_HOME": str(root / "cargo-home"),
                "CARGO_TARGET_DIR": str(output / "target"),
                "PATH": "/usr/bin:/bin",
                "RUSTC": str(compiler),
                "RUSTC_WRAPPER": str(wrapper),
                "THRUM_SELECTED_RUSTC": str(compiler),
                "THRUM_RECEIPT_DIR": str(receipt_dir),
            }
            (root / "cargo-home").mkdir()
            data["command"] = [
                "/usr/bin/sandbox-exec", "-p", "(version 1)(allow default)(deny network*)",
                str(cargo), "build", "--offline", "--manifest-path", str(source / "Cargo.toml"),
            ]
            request.write_text(json.dumps(data))
            runner = (
                "import pathlib,sys;import run_bounded_native_build as native;"
                f"sys.exit(native.run(pathlib.Path({str(request)!r}),"
                f"timeout_seconds={6 if fault == 'timeout' else 120}))"
            )
            supervisor = subprocess.Popen(
                [sys.executable, "-c", runner], cwd=Path(__file__).parent,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            )
            compile_pid = None
            try:
                deadline = time.monotonic() + 12
                while not compile_pidfile.exists() and time.monotonic() < deadline:
                    if supervisor.poll() is not None:
                        break
                    time.sleep(0.01)
                if not compile_pidfile.exists():
                    _, stderr = supervisor.communicate(timeout=6)
                    report = output / "cargo-supervisor.json"
                    self.fail("Cargo did not start a wrapped compile: "
                              + stderr.decode(errors="replace")
                              + (report.read_text() if report.exists() else "no report"))
                compile_pid = int(compile_pidfile.read_text())
                if fault == "sigterm":
                    os.kill(supervisor.pid, signal.SIGTERM)
                _, stderr = supervisor.communicate(timeout=12)
                self.assertEqual(supervisor.returncode, 125,
                                 stderr.decode(errors="replace"))
                report = json.loads((output / "cargo-supervisor.json").read_text())
                self.assertEqual(report["kind"],
                                 "interrupted" if fault == "sigterm" else "supervisorError")
                if fault == "timeout":
                    self.assertEqual(report["detail"], "Cargo build timed out")
                self.assertFalse(report["buildReady"])
                self.assertEqual(report["receiptInventory"]["kind"], "observedOnly")
                self.assertGreaterEqual(report["receiptInventory"]["pending"], 1)
                parsed = [parse_receipt(path.read_bytes(), allow_pending=True)
                          for path in receipt_dir.iterdir()]
                self.assertTrue(any(item.get("pending") and item["rustc"] == os.fsencode(compiler)
                                    for item in parsed))
                deadline = time.monotonic() + 2
                while time.monotonic() < deadline:
                    try:
                        os.kill(compile_pid, 0)
                    except ProcessLookupError:
                        break
                    time.sleep(0.01)
                else:
                    self.fail("same-group Cargo compiler survived supervisor interruption")
            finally:
                if supervisor.poll() is None:
                    supervisor.terminate()
                    try:
                        supervisor.communicate(timeout=6)
                    except subprocess.TimeoutExpired:
                        supervisor.kill()
                        supervisor.communicate()
                if compile_pid is not None:
                    try:
                        os.kill(compile_pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass


if __name__ == "__main__":
    unittest.main()
