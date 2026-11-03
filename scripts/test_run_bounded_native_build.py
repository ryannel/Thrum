"""Candidate native-build supervisor request and result checks."""

import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

from run_bounded_native_build import run


@unittest.skipUnless(sys.platform == "darwin", "first native supervisor profile is macOS")
class NativeBuildSupervisor(unittest.TestCase):
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


if __name__ == "__main__":
    unittest.main()
