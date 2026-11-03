"""macOS candidate checks for append-only sealed tool-root selections."""

from contextlib import contextmanager
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

if sys.platform == "darwin":
    import fcntl

import hold_macos_tool_root_clone as held
from inventory_headless_tool_roots import inventory_root


@contextmanager
def fixture():
    with tempfile.TemporaryDirectory() as directory:
        base = Path(directory)
        source = base / "source"
        (source / "bin").mkdir(parents=True)
        tool = source / "bin" / "tool"
        tool.write_bytes(b"#!/bin/sh\nprintf 'selected\\n'\n")
        tool.chmod(0o755)
        (source / "bin" / "alias").symlink_to("tool")
        parent = base / "owner"
        parent.mkdir(mode=0o700)
        try:
            yield source, parent, inventory_root(source)
        finally:
            # Only fixture teardown clears owner-changeable flags. Production
            # selections are append-only; cleanup needs a separate protocol.
            for directory_path, directories, files in os.walk(parent):
                for name in [*directories, *files]:
                    path = Path(directory_path) / name
                    if not path.is_symlink():
                        os.chflags(path, os.stat(path).st_flags & ~stat.UF_IMMUTABLE)
                        if path.is_dir():
                            path.chmod(0o700)
                        else:
                            path.chmod(0o600)
                os.chflags(directory_path, os.stat(directory_path).st_flags & ~stat.UF_IMMUTABLE)
                os.chmod(directory_path, 0o700)


@unittest.skipUnless(sys.platform == "darwin", "requires macOS file flags and clonefile")
class HoldMacosToolRootClone(unittest.TestCase):
    def test_sealed_executable_and_relative_link_reload(self):
        with fixture() as (source, parent, expected):
            root, record = held.prepare_sealed_clone(source, parent, expected)
            self.assertEqual(record.read_bytes(), held._canonical_record(root.name, expected))
            self.assertTrue(os.stat(record).st_flags & stat.UF_IMMUTABLE)
            with held.open_sealed_clone(parent, expected) as selection:
                selection.assert_live()
                self.assertEqual(selection.root, root)
                self.assertEqual(inventory_root(root), expected)
                self.assertEqual(subprocess.check_output([str(root / "bin" / "alias")]), b"selected\n")
                with self.assertRaises(PermissionError):
                    with (root / "bin" / "tool").open("ab") as stream:
                        stream.write(b"unexpected")
            with self.assertRaisesRegex(ValueError, "closed"):
                selection.assert_live()

    def test_partial_seal_is_never_selected_or_overwritten(self):
        with fixture() as (source, parent, expected):
            original = held._seal_fd
            calls = 0

            def interrupt(fd, is_directory):
                nonlocal calls
                calls += 1
                original(fd, is_directory)
                if calls == 1:
                    raise RuntimeError("interrupted seal")

            with patch.object(held, "_seal_fd", side_effect=interrupt):
                with self.assertRaisesRegex(RuntimeError, "interrupted seal"):
                    held.prepare_sealed_clone(source, parent, expected)
            with self.assertRaises(FileNotFoundError):
                held.open_sealed_clone(parent, expected)
            with self.assertRaisesRegex(ValueError, "already exists or is incomplete"):
                held.prepare_sealed_clone(source, parent, expected)

    def test_record_publish_interruption_refuses(self):
        with fixture() as (source, parent, expected):
            real_seal = held._seal_fd
            record = parent / held._names(expected)[1]

            def interrupt_record(fd, is_directory):
                if record.exists() and os.fstat(fd).st_ino == os.stat(record).st_ino:
                    raise RuntimeError("record seal interrupted")
                real_seal(fd, is_directory)

            with patch.object(held, "_seal_fd", side_effect=interrupt_record):
                with self.assertRaisesRegex(RuntimeError, "record seal interrupted"):
                    held.prepare_sealed_clone(source, parent, expected)
            with self.assertRaisesRegex(ValueError, "untrusted identity"):
                held.open_sealed_clone(parent, expected)
            with self.assertRaisesRegex(ValueError, "already exists or is incomplete"):
                held.prepare_sealed_clone(source, parent, expected)

    def test_modified_record_refuses_cold_selection(self):
        with fixture() as (source, parent, expected):
            _, record = held.prepare_sealed_clone(source, parent, expected)
            os.chflags(record, os.stat(record).st_flags & ~stat.UF_IMMUTABLE)
            record.chmod(0o600)
            record.write_bytes(b"{}\n")
            record_fd = os.open(record, os.O_RDONLY | os.O_NOFOLLOW)
            try:
                held._seal_fd(record_fd, False)
            finally:
                os.close(record_fd)
            with self.assertRaisesRegex(ValueError, "record differs"):
                held.open_sealed_clone(parent, expected)

    def test_two_readers_block_exclusive_preparation(self):
        with fixture() as (source, parent, expected):
            held.prepare_sealed_clone(source, parent, expected)
            _, _, lock_name = held._names(expected)
            lock_path = parent / lock_name
            inode = os.stat(lock_path).st_ino
            self.assertTrue(os.stat(lock_path).st_flags & stat.UF_IMMUTABLE)
            with self.assertRaises(PermissionError):
                lock_path.unlink()
            ready = parent.parent / "second-reader.ready"
            with held.open_sealed_clone(parent, expected):
                child = subprocess.Popen(
                    [sys.executable, "-c",
                     "import sys; from hold_macos_tool_root_clone import open_sealed_clone; "
                     "import json,pathlib; s=open_sealed_clone(sys.argv[1],json.loads(sys.argv[2])); "
                     "pathlib.Path(sys.argv[3]).write_text('ready'); sys.stdin.readline(); s.close()",
                     str(parent), json.dumps(expected), str(ready)],
                    stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
                    text=True,
                    env={**os.environ, "PYTHONPATH": str(Path(__file__).parent)},
                )
                try:
                    deadline = time.monotonic() + 5
                    while not ready.exists() and child.poll() is None and time.monotonic() < deadline:
                        time.sleep(0.01)
                    self.assertTrue(ready.exists(), "second reader did not acquire a lease")
                    with self.assertRaises(BlockingIOError):
                        held._open_lock(parent, lock_name, fcntl.LOCK_EX)
                finally:
                    if child.poll() is None:
                        try:
                            child.stdin.write("\n")
                            child.stdin.flush()
                        except BrokenPipeError:
                            pass
                    try:
                        _, stderr = child.communicate(timeout=5)
                    except subprocess.TimeoutExpired:
                        child.kill()
                        _, stderr = child.communicate(timeout=5)
                        raise AssertionError(f"second reader did not exit: {stderr}")
                    self.assertEqual(child.returncode, 0, stderr)
            lock_fd = held._open_lock(parent, lock_name, fcntl.LOCK_EX)
            os.close(lock_fd)
            self.assertEqual(os.stat(lock_path).st_ino, inode)

    def test_owner_exit_does_not_remove_surviving_child_root(self):
        with fixture() as (source, parent, expected):
            root, _ = held.prepare_sealed_clone(source, parent, expected)
            output = parent.parent / "surviving-child.out"
            started = parent.parent / "surviving-child.started"
            worker_code = (
                "import os,pathlib,subprocess,sys; "
                "pathlib.Path(sys.argv[3]).write_text('started'); sys.stdin.readline(); "
                "out=pathlib.Path(sys.argv[2]); tmp=out.with_suffix('.pending'); "
                "tmp.write_bytes(subprocess.check_output([sys.argv[1]])); os.replace(tmp,out)"
            )
            owner_code = (
                "import json,os,sys; "
                "from hold_macos_tool_root_clone import open_sealed_clone; "
                "lease=open_sealed_clone(sys.argv[1],json.loads(sys.argv[2])); "
                "os._exit(0)"
            )
            worker = subprocess.Popen(
                [sys.executable, "-c", worker_code, str(root / "bin" / "alias"),
                 str(output), str(started)],
                stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
                text=True,
            )
            try:
                deadline = time.monotonic() + 5
                while not started.exists() and worker.poll() is None and time.monotonic() < deadline:
                    time.sleep(0.01)
                self.assertTrue(started.exists(), "controlled worker did not start")
                subprocess.run(
                    [sys.executable, "-c", owner_code, str(parent), json.dumps(expected)],
                    check=True, timeout=5,
                    env={**os.environ, "PYTHONPATH": str(Path(__file__).parent)},
                )
                self.assertIsNone(worker.poll(), "worker exited before lease owner")
                with self.assertRaisesRegex(ValueError, "already exists or is incomplete"):
                    held.prepare_sealed_clone(source, parent, expected)
            finally:
                if worker.poll() is None:
                    try:
                        worker.stdin.write("\n")
                        worker.stdin.flush()
                    except BrokenPipeError:
                        pass
                try:
                    _, stderr = worker.communicate(timeout=5)
                except subprocess.TimeoutExpired:
                    worker.kill()
                    _, stderr = worker.communicate(timeout=5)
                    raise AssertionError(f"controlled worker did not exit: {stderr}")
                self.assertEqual(worker.returncode, 0, stderr)
            self.assertEqual(output.read_bytes(), b"selected\n")
            with held.open_sealed_clone(parent, expected) as selection:
                selection.assert_live()


if __name__ == "__main__":
    unittest.main()
