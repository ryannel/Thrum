#!/usr/bin/env python3
"""Exercise the merge cache with disposable Git worktrees and a small check."""

import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

if sys.platform == "darwin":
    import fcntl
    import merge_verify as verify


@unittest.skipUnless(sys.platform == "darwin", "the merge verification cache runs on macOS")
class MergeVerification(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name).resolve()
        self.cache = self.root / ".git/orchestrate/merge-cargo"
        self.checkout = self.cache / "checkout"
        self.target = self.checkout / "target"
        self.env = {key: value for key, value in os.environ.items()
                    if not key.startswith(("THRUM_", "GIT_", "CARGO_"))}
        self.env.update(GIT_AUTHOR_NAME="Merge Test", GIT_AUTHOR_EMAIL="merge@example.invalid",
                        GIT_COMMITTER_NAME="Merge Test", GIT_COMMITTER_EMAIL="merge@example.invalid")
        self.git("init", "--quiet", "-b", "main")
        (self.root / ".gitignore").write_text("/target/\n/local.cfg\n")
        (self.root / "input").write_text("pass\n")
        dev = self.root / "dev"
        dev.write_text("""#!/bin/sh
set -eu
echo "check cwd=$PWD"
test "$1" = check
mkdir -p target/nextest
echo artifact > target/artifact
touch target/nextest/run-report
if test "$(cat input)" = fail; then
  echo 'local or committed assertion failed'
  exit 37
fi
""")
        dev.chmod(0o755)
        self.git("add", ".")
        self.git("commit", "--quiet", "-m", "fixture")
        self.git("update-ref", verify.MAIN, "HEAD")
        self.base = self.git("rev-parse", "HEAD")

    def git(self, *args, root=None):
        return subprocess.check_output(["git", *args], cwd=root or self.root, env=self.env, text=True).strip()

    def run_check(self, *, root=None, **env):
        runner = ("import sys; sys.path.insert(0, sys.argv[1]); import merge_verify as v;"
                  " v.FREE_FLOOR = v.COLD_HEADROOM = v.WARM_HEADROOM = 0;"
                  " v.check_cargo_layout = lambda root: None; sys.exit(v.main())")
        return subprocess.run([sys.executable, "-c", runner, str(Path(verify.__file__).parent)],
                              cwd=root or self.root, text=True,
                              capture_output=True, env=self.env | env, timeout=10)

    def assert_check(self, result, status, *lines):
        self.assertEqual(result.returncode, status, result.stdout + result.stderr)
        for line in lines:
            self.assertIn(line, result.stdout)

    def test_candidate_verdict_and_isolated_outputs(self):
        self.assert_check(self.run_check(), 0, "cold", f"base={self.base}", f"check cwd={self.checkout}")
        (self.root / "input").write_text("fail\n")
        self.git("commit", "--quiet", "-am", "failing candidate")
        failed = self.git("rev-parse", "HEAD")
        self.assert_check(self.run_check(), 37, f"warm candidate={failed}", f"base={self.base}",
                          "local or committed assertion failed")
        self.git("revert", "--no-edit", "HEAD")
        self.assert_check(self.run_check(), 0, "warm", f"check cwd={self.checkout}")
        self.assertTrue((self.target / "artifact").is_file())
        self.assertFalse((self.target / "nextest/run-report").exists())
        self.assertFalse((self.root / "target").exists())

    def test_caller_inputs_take_fallback(self):
        self.assert_check(self.run_check(), 0, "cold")
        input_path = self.root / "input"
        input_path.write_text("fail\n")
        self.assert_check(self.run_check(), 37, f"ordinary check in {self.root}", "assertion failed")
        self.git("add", "input")
        self.assert_check(self.run_check(), 37, "staged, unstaged or untracked caller changes")
        self.git("restore", "--source=HEAD", "--staged", "--worktree", "input")
        (self.root / "extra").touch()
        self.assert_check(self.run_check(), 0, "staged, unstaged or untracked caller changes")
        (self.root / "extra").unlink()
        (self.root / "local.cfg").touch()
        self.assert_check(self.run_check(), 0, "checkout-local ignored path local.cfg")
        (self.root / "local.cfg").unlink()
        self.git("update-index", "--assume-unchanged", "input")
        input_path.write_text("fail\n")
        self.assert_check(self.run_check(), 37, "index contains sparse, assumed-unchanged or submodule paths")
        self.git("update-index", "--no-assume-unchanged", "input")
        input_path.write_text("pass\n")
        self.assert_check(self.run_check(), 0, "warm", f"check cwd={self.checkout}")
        self.assert_check(self.run_check(THRUM_MERGE_CACHE="0"), 0, "disabled", f"check cwd={self.root}")

    def test_busy_cache_and_interrupted_run(self):
        self.assert_check(self.run_check(), 0, "cold")
        artifact = self.target / "unfinished"
        artifact.touch()
        (self.cache / "incomplete").touch()
        with (self.cache / "lock").open("r+b") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            self.assert_check(self.run_check(), 0, "busy", f"check cwd={self.root}")
            self.assertTrue(artifact.is_file())
        self.assert_check(self.run_check(), 0, "reset (interrupted", "cold", f"check cwd={self.checkout}")
        self.assertFalse(artifact.exists())
        self.assertTrue((self.root / "target/artifact").is_file())

    def test_inherited_tool_configuration_takes_fallback(self):
        parent = self.root / "callers"
        parent.mkdir()
        caller = parent / "singleton"
        self.git("worktree", "add", "--quiet", "--detach", str(caller), self.base)
        self.assert_check(self.run_check(root=caller), 0, "cold", f"check cwd={self.checkout}")
        for tool, relative in (("rustfmt", "rustfmt.toml"), ("rustfmt", ".rustfmt.toml"),
                               ("Clippy", "clippy.toml"), ("Clippy", ".clippy.toml"),
                               ("Cargo", ".cargo/config.toml")):
            with self.subTest(configuration=relative):
                config = parent / relative
                config.parent.mkdir(exist_ok=True)
                config.touch()
                self.assert_check(self.run_check(root=caller), 0, f"inherited {tool} configuration",
                                  f"check cwd={caller}")
                config.unlink()
        self.assert_check(self.run_check(root=caller), 0, "warm", f"check cwd={self.checkout}")

    def test_checkout_relative_tools_and_opaque_flags_take_fallback(self):
        self.assert_check(self.run_check(), 0, "cold")
        overrides = {
            "CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER": "./tmp/linker",
            "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER": "./tmp/wasm-linker",
            "CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER": "/usr/bin/env ./tmp/runner",
            "CARGO_BUILD_RUSTC_WRAPPER": "./tmp/rustc",
            "RUSTFMT": "./tmp/rustfmt",
            "RUSTFLAGS": "-L ./tmp/native",
        }
        for key, value in overrides.items():
            with self.subTest(override=key):
                self.assert_check(self.run_check(**{key: value}), 0, key, f"check cwd={self.root}")
        self.assert_check(self.run_check(CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER="/usr/bin/cc"),
                          0, "warm", f"check cwd={self.checkout}")

    def test_tracked_configurations_can_reference_caller_files(self):
        caller = self.root / "singleton"
        self.git("worktree", "add", "--quiet", "--detach", str(caller), self.base)
        self.assert_check(self.run_check(root=caller), 0, "cold")
        (caller / "target").mkdir()
        (caller / "target/local.cfg").touch()
        (caller / "rustfmt.toml").symlink_to("target/local.cfg")
        self.git("add", "rustfmt.toml", root=caller)
        self.git("commit", "--quiet", "-m", "Use an external formatter config", root=caller)
        self.assert_check(self.run_check(root=caller), 0, "configuration symlink rustfmt.toml",
                          f"check cwd={caller}")
        self.git("checkout", "--quiet", "--detach", self.base, root=caller)
        config = caller / ".cargo/config.toml"
        config.parent.mkdir()
        config.write_text('[target.aarch64-apple-darwin]\nlinker = "./target/local.cfg"\n')
        self.git("add", ".cargo", root=caller)
        self.git("commit", "--quiet", "-m", "Select a checkout-local tool", root=caller)
        self.assert_check(self.run_check(root=caller), 0, "checkout-local Cargo tool or build configuration",
                          f"check cwd={caller}")

    def test_retention_and_low_space_reclaim_only_cache_artifacts(self):
        self.assert_check(self.run_check(), 0, "cold")
        ordinary = self.root / "target"
        ordinary.mkdir()
        (ordinary / "keep").touch()
        (self.target / "oversized").write_bytes(b"full" * 4096)
        with patch.object(verify, "CAP", 0), patch.object(verify, "FREE_FLOOR", 0):
            verify.admit_space(self.cache, self.checkout)
        self.assertFalse(self.target.exists())
        self.target.mkdir()
        (self.target / "old").write_bytes(b"old")
        with patch.object(verify, "FREE_FLOOR", 10 ** 20):
            with self.assertRaisesRegex(verify.Unavailable, "low free space"):
                verify.admit_space(self.cache, self.checkout)
        self.assertFalse(self.target.exists())
        self.assertTrue((ordinary / "keep").is_file())


if __name__ == "__main__":
    unittest.main()
