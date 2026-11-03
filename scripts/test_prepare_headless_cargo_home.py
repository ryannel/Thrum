"""Failure-boundary checks for first-profile private Cargo cache preparation."""

from pathlib import Path
import hashlib
import json
import os
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock

from prepare_headless_cargo_home import (
    ChildExitObserver, cargo_reuse_key, collect_bounded, possible_headless_packages, prepare,
    root_inventory,
)


class PrepareHeadlessCargoHome(unittest.TestCase):
    @unittest.skipUnless(sys.platform == "darwin", "kqueue fast-exit race is macOS")
    def test_fast_child_exits_before_kqueue_registration(self):
        def delayed_observer(process):
            time.sleep(0.05)
            return ChildExitObserver(process)

        with tempfile.TemporaryDirectory() as directory:
            with mock.patch("prepare_headless_cargo_home.ChildExitObserver",
                            side_effect=delayed_observer):
                status, output, error = collect_bounded(
                    ["/usr/bin/true"], Path(directory), os.environ.copy(),
                )
            self.assertEqual((status, output, error), (0, b"", b""))

    @unittest.skipUnless(os.name == "posix", "private inventory requires POSIX")
    def test_private_inventory_preserves_digest_order_and_refuses_parent_swap(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "cache"
            root.mkdir()
            (root / "a").mkdir()
            (root / "a/x").write_bytes(b"x")
            (root / "a.b").write_bytes(b"y")
            self.assertEqual(root_inventory(root), {
                "sha256": "c80e10e25120c20ba77311c33856b5781d599e7ae4f9af0f1664e9b24b31e1d3",
                "files": 2, "bytes": 2,
            })
            outside = Path(directory) / "outside"
            outside.mkdir()
            (outside / "x").write_bytes(b"outside")
            real_open = os.open
            swapped = False

            def swap_parent(path, flags, *args, **kwargs):
                nonlocal swapped
                if path == "a" and kwargs.get("dir_fd") is not None and not swapped:
                    swapped = True
                    (root / "a").rename(root / "old")
                    (root / "a").symlink_to(outside, target_is_directory=True)
                return real_open(path, flags, *args, **kwargs)

            with mock.patch("prepare_headless_cargo_home.os.open", side_effect=swap_parent):
                with self.assertRaises(OSError):
                    root_inventory(root)
            self.assertTrue(swapped)

    @unittest.skipUnless(os.name == "posix", "private inventory requires POSIX")
    def test_private_inventory_refuses_fifo_without_blocking(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "cache"
            root.mkdir()
            os.mkfifo(root / "replacement.crate")
            started = time.monotonic()
            with self.assertRaisesRegex(ValueError, "non-file entry"):
                root_inventory(root)
            self.assertLess(time.monotonic() - started, 1)

    @unittest.skipUnless(os.name == "posix", "process groups require POSIX")
    def test_interruption_reaps_metadata_child_before_cleanup(self):
        with tempfile.TemporaryDirectory() as directory:
            pidfile = Path(directory) / "child.pid"
            child_code = (
                "import os,pathlib,time;"
                f"p=pathlib.Path({str(pidfile)!r});"
                "tmp=p.with_suffix('.tmp');tmp.write_text(str(os.getpid()));tmp.replace(p);"
                "time.sleep(10)"
            )
            runner = (
                "import os,sys;"
                f"sys.path.insert(0,{str(Path(__file__).parent)!r});"
                "from prepare_headless_cargo_home import collect_bounded;"
                f"collect_bounded([sys.executable,'-c',{child_code!r}],"
                f"{directory!r},os.environ.copy())"
            )
            wrapper = subprocess.Popen([sys.executable, "-c", runner],
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            child_pid = None
            try:
                deadline = time.monotonic() + 3
                while not pidfile.exists() and time.monotonic() < deadline:
                    time.sleep(0.01)
                self.assertTrue(pidfile.exists(), "metadata child did not start")
                child_pid = int(pidfile.read_text())
                os.kill(wrapper.pid, signal.SIGINT)
                wrapper.communicate(timeout=4)
                self.assertNotEqual(wrapper.returncode, 0)
                with self.assertRaises(ProcessLookupError):
                    os.kill(child_pid, 0)
            finally:
                if wrapper.poll() is None:
                    wrapper.kill()
                    wrapper.communicate()
                if child_pid is not None:
                    try:
                        os.kill(child_pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass

    @unittest.skipUnless(os.name == "posix", "early signal deferral requires POSIX")
    def test_signal_during_popen_return_reaps_new_child(self):
        with tempfile.TemporaryDirectory() as directory:
            pidfile = Path(directory) / "child.pid"
            runner = f"""
import os,pathlib,signal,subprocess,sys
sys.path.insert(0, {str(Path(__file__).parent)!r})
import prepare_headless_cargo_home as native
real_popen = subprocess.Popen
def launch(*args, **kwargs):
    child = real_popen(*args, **kwargs)
    pathlib.Path({str(pidfile)!r}).write_text(str(child.pid))
    os.kill(os.getpid(), signal.SIGTERM)
    return child
def interrupted(*_):
    raise KeyboardInterrupt()
signal.signal(signal.SIGTERM, interrupted)
native.subprocess.Popen = launch
native.collect_bounded([sys.executable, '-c', 'import time; time.sleep(30)'],
                       {directory!r}, os.environ.copy())
"""
            wrapper = subprocess.Popen([sys.executable, "-c", runner],
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            child_pid = None
            try:
                _, stderr = wrapper.communicate(timeout=5)
                self.assertNotEqual(wrapper.returncode, 0, stderr.decode(errors="replace"))
                self.assertTrue(pidfile.exists(), "Popen did not return a child")
                child_pid = int(pidfile.read_text())
                deadline = time.monotonic() + 2
                while time.monotonic() < deadline:
                    try:
                        os.kill(child_pid, 0)
                    except ProcessLookupError:
                        break
                    time.sleep(0.01)
                else:
                    self.fail("early SIGTERM left the launched child alive")
            finally:
                if wrapper.poll() is None:
                    wrapper.kill()
                    wrapper.communicate()
                if child_pid is not None:
                    try:
                        os.kill(child_pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass

    @unittest.skipUnless(os.name == "posix", "child signal delivery requires POSIX")
    def test_collector_does_not_block_child_sigterm(self):
        with tempfile.TemporaryDirectory() as directory:
            status, _, _ = collect_bounded(
                [sys.executable, "-c",
                 "import os,signal; os.kill(os.getpid(), signal.SIGTERM)"],
                Path(directory), os.environ.copy(), timeout_seconds=3,
            )
            self.assertEqual(status, -signal.SIGTERM)

    @unittest.skipUnless(os.name == "posix", "ignored child signal disposition requires POSIX")
    def test_collector_preserves_ignored_child_sigint(self):
        prior = signal.getsignal(signal.SIGINT)
        signal.signal(signal.SIGINT, signal.SIG_IGN)
        try:
            with tempfile.TemporaryDirectory() as directory:
                status, output, _ = collect_bounded(
                    [sys.executable, "-c",
                     "import signal,sys; sys.stdout.write(str(signal.getsignal(signal.SIGINT) "
                     "== signal.SIG_IGN))"],
                    Path(directory), os.environ.copy(), timeout_seconds=3,
                )
            self.assertEqual((status, output), (0, b"True"))
        finally:
            signal.signal(signal.SIGINT, prior)

    @unittest.skipUnless(os.name == "posix", "private inventory requires POSIX")
    def test_verify_reuse_refuses_dependency_selection_change(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "source"
            source.mkdir()
            (source / "Cargo.toml").write_text("[workspace]\n")
            (source / "Cargo.lock").write_text("version = 4\n")
            (source / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "1.99.1"\n')
            (source / "game.rs").write_text("fn a() {}\n")
            cargo = Path(directory) / "cargo"
            rustc = Path(directory) / "rustc"
            cargo.write_bytes(b"cargo")
            rustc.write_bytes(b"rustc")
            tools = {
                name: {"path": str(path.resolve()), "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                       "bytes": path.stat().st_size}
                for name, path in (("cargo", cargo), ("rustc", rustc))
            }
            target = "aarch64-apple-darwin"
            private_home = Path(directory) / "private-home"
            private_home.mkdir()
            cached = private_home / "cached.crate"
            cached.write_bytes(b"accepted dependency")
            candidate = Path(directory) / "candidate.json"
            candidate.write_text(json.dumps({
                "format": 2, "target": target, "toolExecutables": tools,
                "reuseKey": cargo_reuse_key(source, target, tools),
                "privateRoot": root_inventory(private_home),
            }))
            command = [sys.executable, str(Path(__file__).with_name("prepare_headless_cargo_home.py")),
                       "--source", str(source), "--verify-reuse", str(candidate),
                       "--private-home", str(private_home),
                       "--target", target, "--cargo", str(cargo), "--rustc", str(rustc)]
            self.assertEqual(subprocess.run(command, capture_output=True).returncode, 0)
            relative_source = command.copy()
            relative_source[relative_source.index("--source") + 1] = "source"
            self.assertIn(
                b"must be absolute",
                subprocess.run(relative_source, cwd=directory, capture_output=True).stderr,
            )
            wrong_target = command.copy()
            wrong_target[wrong_target.index("--target") + 1] = "x86_64-apple-darwin"
            self.assertIn(b"target differs", subprocess.run(wrong_target, capture_output=True).stderr)
            rustc.write_bytes(b"changed")
            self.assertIn(b"tool executables differ", subprocess.run(command, capture_output=True).stderr)
            rustc.write_bytes(b"rustc")
            (source / "game.rs").write_text("fn b() {}\n")
            self.assertEqual(subprocess.run(command, capture_output=True).returncode, 0)
            cached.write_bytes(b"changed dependency")
            self.assertIn(b"cache bytes changed", subprocess.run(command, capture_output=True).stderr)
            cached.write_bytes(b"accepted dependency")
            (source / "Cargo.toml").write_text("[workspace]\nresolver = '3'\n")
            self.assertIn(b"cannot be reused", subprocess.run(command, capture_output=True).stderr)
            (source / "Cargo.toml").write_text("[workspace]\n")
            (source / "build.rs").write_text("fn main() {}\n")
            self.assertIn(b"implicit build scripts", subprocess.run(command, capture_output=True).stderr)
            (source / "build.rs").unlink()
            (source / ".cargo").mkdir()
            (source / ".cargo/config.toml").write_text("[build]\n")
            self.assertIn(b"ambient Cargo configuration", subprocess.run(command, capture_output=True).stderr)

    def test_cache_reuse_key_tracks_manifests_without_charging_code_edits(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory)
            (source / "Cargo.toml").write_text("[workspace]\n")
            (source / "Cargo.lock").write_text("version = 4\n")
            (source / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "1.99.1"\n')
            (source / "game.rs").write_text("fn a() {}\n")
            tools = {"cargo": {"sha256": "a" * 64, "bytes": 1},
                     "rustc": {"sha256": "b" * 64, "bytes": 1}}
            first = cargo_reuse_key(source, "aarch64-apple-darwin", tools)
            relocated = {name: {**identity, "path": f"/different/{name}"}
                         for name, identity in tools.items()}
            self.assertEqual(first, cargo_reuse_key(source, "aarch64-apple-darwin", relocated))
            (source / "App").mkdir()
            (source / "App/Cargo.toml").write_text("[package]\nname = 'app'\n")
            with_child = cargo_reuse_key(source, "aarch64-apple-darwin", tools)
            self.assertNotEqual(first["sha256"], with_child["sha256"])
            (source / "App/Cargo.toml").unlink()
            (source / "App").rmdir()
            (source / "game.rs").write_text("fn b() {}\n")
            self.assertEqual(first, cargo_reuse_key(source, "aarch64-apple-darwin", tools))
            (source / "Cargo.toml").write_text("[workspace]\nresolver = '3'\n")
            self.assertNotEqual(first["sha256"], cargo_reuse_key(source, "aarch64-apple-darwin", tools)["sha256"])
            (source / "Cargo.toml").write_text("[workspace]\n")
            self.assertNotEqual(first["sha256"], cargo_reuse_key(source, "x86_64-apple-darwin", tools)["sha256"])

    def test_selected_package_walk_ignores_dev_and_refuses_build_hooks(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory)
            manifest = source / "examples/i1-mobile-game/Cargo.toml"
            manifest.parent.mkdir(parents=True)
            manifest.write_text("[package]\nname = 'i1-mobile-game'\n")
            packages = [
                {"id": name, "name": name, "version": "0.1.0", "source": None,
                 "manifest_path": str(source / path / "Cargo.toml"),
                 "targets": [{"kind": ["lib"]}]}
                for name, path in (
                    ("i1-mobile-game", "examples/i1-mobile-game"),
                    ("core", "crates/core"), ("build-helper", "crates/build-helper"),
                    ("dev-only", "crates/dev-only"),
                )
            ]
            metadata = {"packages": packages, "resolve": {"nodes": [
                {"id": "i1-mobile-game", "features": [], "deps": [
                    {"pkg": "core", "dep_kinds": [{"kind": None}]},
                    {"pkg": "build-helper", "dep_kinds": [{"kind": "build"}]},
                    {"pkg": "dev-only", "dep_kinds": [{"kind": "dev"}]},
                ]},
                *({"id": name, "features": [], "deps": []}
                  for name in ("core", "build-helper", "dev-only")),
            ]}}
            selected = possible_headless_packages(metadata, source)
            self.assertEqual({package["name"] for package in selected},
                             {"i1-mobile-game", "core", "build-helper"})
            packages[1]["targets"] = [{"kind": ["custom-build"]}]
            with self.assertRaisesRegex(ValueError, "executable dependency hooks"):
                possible_headless_packages(metadata, source)

    def test_metadata_output_is_stopped_during_collection(self):
        with tempfile.TemporaryDirectory() as directory:
            started = time.monotonic()
            with self.assertRaisesRegex(ValueError, "output limit"):
                collect_bounded(
                    [sys.executable, "-c", "import sys,time;sys.stdout.buffer.write(b'x'*2048);sys.stdout.flush();time.sleep(10)"],
                    Path(directory), os.environ.copy(), stdout_limit=1024,
                )
            self.assertLess(time.monotonic() - started, 2)

    @unittest.skipUnless(os.name == "posix", "process groups require POSIX")
    def test_metadata_descendant_cannot_hold_output_pipes(self):
        with tempfile.TemporaryDirectory() as directory:
            started = time.monotonic()
            with self.assertRaisesRegex(ValueError, "descendants retained output pipes"):
                collect_bounded(
                    [sys.executable, "-c", (
                        "import subprocess,sys;"
                        "subprocess.Popen([sys.executable,'-c','import time;time.sleep(10)'])"
                    )],
                    Path(directory), os.environ.copy(),
                )
            self.assertLess(time.monotonic() - started, 3)

    @unittest.skipUnless(os.name == "posix", "sessions require POSIX")
    def test_escaped_session_cannot_block_collector_cleanup(self):
        with tempfile.TemporaryDirectory() as directory:
            pidfile = Path(directory) / "escaped.pid"
            escaped = (
                "import os,pathlib,time;"
                f"marker=pathlib.Path({str(pidfile)!r});"
                "temp=marker.with_suffix('.tmp');"
                "temp.write_text(str(os.getpid()));temp.replace(marker);"
                "time.sleep(10)"
            )
            launcher = (
                "import subprocess,sys;"
                f"subprocess.Popen([sys.executable,'-c',{escaped!r}],"
                "start_new_session=True)"
            )
            escaped_pid = None
            try:
                started = time.monotonic()
                with self.assertRaisesRegex(ValueError, "descendants retained output pipes"):
                    collect_bounded(
                        [sys.executable, "-c", launcher],
                        Path(directory), os.environ.copy(),
                    )
                self.assertLess(time.monotonic() - started, 3)
                escaped_pid = int(pidfile.read_text())
            finally:
                if escaped_pid is None and pidfile.exists():
                    escaped_pid = int(pidfile.read_text())
                if escaped_pid is not None:
                    try:
                        os.kill(escaped_pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass

    @unittest.skipUnless(os.name == "posix", "process groups require POSIX")
    def test_closed_pipe_descendant_is_stopped_before_success(self):
        with tempfile.TemporaryDirectory() as directory:
            pidfile = Path(directory) / "descendant.pid"
            child = (
                "import os,pathlib,time;"
                f"marker=pathlib.Path({str(pidfile)!r});"
                "temp=marker.with_suffix('.tmp');"
                "temp.write_text(str(os.getpid()));temp.replace(marker);"
                "time.sleep(10)"
            )
            launcher = (
                "import pathlib,subprocess,sys,time;"
                f"marker=pathlib.Path({str(pidfile)!r});"
                f"subprocess.Popen([sys.executable,'-c',{child!r}],"
                "stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,"
                "stderr=subprocess.DEVNULL);"
                "end=time.monotonic()+3;"
                "\nwhile not marker.exists() and time.monotonic()<end: time.sleep(0.01)"
                "\nif not marker.exists(): raise SystemExit('descendant did not start')"
            )
            child_pid = None
            try:
                status, _, _ = collect_bounded(
                    [sys.executable, "-c", launcher],
                    Path(directory), os.environ.copy(),
                )
                self.assertEqual(status, 0)
                child_pid = int(pidfile.read_text())
                deadline = time.monotonic() + 3
                while time.monotonic() < deadline:
                    try:
                        os.kill(child_pid, 0)
                    except ProcessLookupError:
                        break
                    time.sleep(0.02)
                else:
                    self.fail("closed-pipe descendant survived completed collection")
            finally:
                if child_pid is None and pidfile.exists():
                    child_pid = int(pidfile.read_text())
                if child_pid is not None:
                    try:
                        os.kill(child_pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass

    @unittest.skipUnless(os.name == "posix", "pipe fixture requires POSIX")
    def test_metadata_child_receives_closed_stdin(self):
        with tempfile.TemporaryDirectory() as directory:
            child = "import sys;print(len(sys.stdin.buffer.read(1)))"
            runner = (
                "import os,sys;"
                f"sys.path.insert(0,{str(Path(__file__).parent)!r});"
                "from prepare_headless_cargo_home import collect_bounded;"
                f"result=collect_bounded([sys.executable,'-c',{child!r}],"
                f"{directory!r},os.environ.copy(),timeout_seconds=1);"
                "sys.stdout.buffer.write(result[1])"
            )
            wrapper = subprocess.Popen(
                [sys.executable, "-c", runner], stdin=subprocess.PIPE,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            )
            try:
                self.assertEqual(wrapper.wait(timeout=3), 0)
                self.assertEqual(wrapper.stdout.read(), b"0\n")
            finally:
                if wrapper.poll() is None:
                    wrapper.kill()
                    wrapper.wait()
                wrapper.stdin.close()
                wrapper.stdout.close()
                wrapper.stderr.close()

    @unittest.skipUnless(os.name == "posix", "parent identity requires POSIX")
    def test_supervisor_stops_child_when_parent_disappears(self):
        with tempfile.TemporaryDirectory() as directory:
            pidfile = Path(directory) / "child.pid"
            child = (
                "import os,pathlib,time;"
                f"marker=pathlib.Path({str(pidfile)!r});"
                "temp=marker.with_suffix('.tmp');"
                "temp.write_text(str(os.getpid()));temp.replace(marker);"
                "time.sleep(10)"
            )
            supervisor = (
                "import os,sys;"
                f"sys.path.insert(0,{str(Path(__file__).parent)!r});"
                "from prepare_headless_cargo_home import collect_bounded;"
                f"collect_bounded([sys.executable,'-c',{child!r}],"
                f"{directory!r},os.environ.copy(),parent_pid=os.getppid())"
            )
            launcher = (
                "import pathlib,subprocess,sys,time;"
                f"p=subprocess.Popen([sys.executable,'-c',{supervisor!r}],"
                "stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);"
                f"marker=pathlib.Path({str(pidfile)!r});"
                "end=time.monotonic()+3;"
                "\nwhile not marker.exists() and time.monotonic()<end: time.sleep(0.01)"
                "\nif not marker.exists(): raise SystemExit('child did not start')"
                "\nprint(p.pid,flush=True)"
            )
            supervisor_pid = None
            child_pid = None
            try:
                launched = subprocess.run(
                    [sys.executable, "-c", launcher],
                    capture_output=True, timeout=5, check=True,
                )
                supervisor_pid = int(launched.stdout)
                child_pid = int(pidfile.read_text())
                deadline = time.monotonic() + 3
                while time.monotonic() < deadline:
                    try:
                        os.kill(child_pid, 0)
                    except ProcessLookupError:
                        break
                    time.sleep(0.02)
                else:
                    self.fail("child survived supervisor parent loss")
            finally:
                for pid in (child_pid, supervisor_pid):
                    if pid is not None:
                        try:
                            os.kill(pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass

    def fixture(self, root):
        source = root / "accepted-input"
        source.mkdir()
        (source / "Cargo.toml").write_text("[workspace]\n")
        (source / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "1.99.1"\n')
        shared = root / "shared-home"
        shared.mkdir()
        tools = root / "tools"
        tools.mkdir()
        cargo = tools / "cargo"
        rustc = tools / "rustc"
        cargo.write_bytes(b"fixture")
        rustc.write_bytes(b"fixture")
        return source, shared, cargo, rustc, root / "private-home"

    def test_bad_lock_archive_refuses_and_removes_only_new_cache(self):
        with tempfile.TemporaryDirectory() as directory:
            source, shared, cargo, rustc, destination = self.fixture(Path(directory))
            (source / "Cargo.lock").write_text(
                'version = 4\n[[package]]\nname = "hashbrown"\nversion = "0.17.1"\n'
                'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
                'checksum = "' + ("0" * 64) + '"\n'
            )
            registry = "index.crates.io-fixture"
            index = shared / "registry/index" / registry
            (index / ".cache/ha/sh").mkdir(parents=True)
            (index / "config.json").write_bytes(b"{}")
            (index / ".cache/ha/sh/hashbrown").write_bytes(b"index fixture")
            archive = shared / "registry/cache" / registry
            archive.mkdir(parents=True)
            (archive / "hashbrown-0.17.1.crate").write_bytes(b"wrong archive")

            with self.assertRaisesRegex(ValueError, "checksum mismatch"):
                prepare(source, shared, destination, cargo, rustc, "aarch64-apple-darwin")
            self.assertFalse(destination.exists())
            self.assertFalse(destination.with_name("private-home.manifest.json").exists())
            self.assertTrue((source / "Cargo.lock").is_file())

    def test_ambient_cargo_config_refuses_before_creating_cache(self):
        with tempfile.TemporaryDirectory() as directory:
            source, shared, cargo, rustc, destination = self.fixture(Path(directory))
            config = source / ".cargo/config.toml"
            config.parent.mkdir()
            config.write_text('[build]\nrustc-workspace-wrapper = "/usr/bin/false"\n')

            with self.assertRaisesRegex(ValueError, "ambient Cargo configuration"):
                prepare(source, shared, destination, cargo, rustc, "aarch64-apple-darwin")
            self.assertTrue(config.is_file())
            self.assertFalse(destination.exists())

    def test_lock_version_cannot_escape_cache_path(self):
        with tempfile.TemporaryDirectory() as directory:
            source, shared, cargo, rustc, destination = self.fixture(Path(directory))
            (source / "Cargo.lock").write_text(
                'version = 4\n[[package]]\nname = "hashbrown"\n'
                'version = "../outside"\n'
                'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
                'checksum = "' + ("0" * 64) + '"\n'
            )
            with self.assertRaisesRegex(ValueError, "unsupported registry package spelling"):
                prepare(source, shared, destination, cargo, rustc, "aarch64-apple-darwin")
            self.assertFalse(destination.exists())
            self.assertFalse((Path(directory) / "outside.crate").exists())


if __name__ == "__main__":
    unittest.main()
