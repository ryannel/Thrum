"""Real offline Cargo builds through the gate in isolated Git repositories."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class GameGate(unittest.TestCase):
    def setUp(self):
        scratch = ROOT / "tmp/game-gate-tests"
        scratch.mkdir(parents=True, exist_ok=True)
        self.directory = tempfile.TemporaryDirectory(dir=scratch)
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name).resolve()
        self.engine = self.root / "thrum"
        self.games = self.root / "thrum-games"
        self.write(self.engine / "Cargo.toml", '[workspace]\nmembers=["crates/*", "examples/demo"]\nresolver="2"\n')
        self.write(self.engine / "crates/engine/Cargo.toml", '[package]\nname="engine"\nversion="0.1.0"\nedition="2021"\n')
        self.write(self.engine / "crates/engine/src/lib.rs", 'pub fn identity() -> u8 { 1 }\n')
        for folder, name in (("crates/thrum-fixtures", "thrum-fixtures"),
                             ("crates/thrum-test-support", "thrum-test-support"),
                             ("examples/demo", "demo")):
            self.write(self.engine / folder / "Cargo.toml", f'[package]\nname="{name}"\nversion="0.1.0"\nedition="2021"\n')
            self.write(self.engine / folder / "src/lib.rs", "")
        self.write(self.engine / ".config/architecture.toml", '''[packages.engine]
manifest="crates/engine/Cargo.toml"
[packages.demo]
manifest="examples/demo/Cargo.toml"
[packages.thrum-fixtures]
manifest="crates/thrum-fixtures/Cargo.toml"
test_role="fixtures"
[packages.thrum-test-support]
manifest="crates/thrum-test-support/Cargo.toml"
test_role="support"
[boundaries]
proving_ground="scratch-game"
''')
        for script in ("game_gate.py", "shape_rules.py", "shape_matchers.py", "evidence_store.py"):
            self.write(self.engine / "scripts" / script, (ROOT / "scripts" / script).read_text())
        self.init(self.engine)
        self.manifest = '[package]\nname="scratch-game"\nversion="0.1.0"\nedition="2021"\n[dependencies]\nengine={path="../../thrum/crates/engine"}\n'
        self.write(self.games / "sample/Cargo.toml", self.manifest)
        self.write(self.games / "sample/src/lib.rs", '#[test] fn checkout() { assert_eq!(engine::identity(), 2); }\n')
        self.write(self.games / "sample/scripts/game.py", 'import sys\nassert sys.argv[1:] == ["echo", "--literal", "two words"]\n')
        self.write(self.games / "sample/Cargo.lock", '''version = 4
[[package]]
name = "engine"
version = "0.1.0"
[[package]]
name = "scratch-game"
version = "0.1.0"
dependencies = ["engine"]
''')
        self.init(self.games)
        self.worktree = self.root / "builder"
        self.git(self.engine, "worktree", "add", "-qb", "builder", str(self.worktree))
        self.write(self.worktree / "crates/engine/src/lib.rs", 'pub fn identity() -> u8 { 2 }\n')
        self.commit(self.worktree)
        self.env = dict(os.environ)
        self.env.pop("THRUM_GAMES", None)
        self.record = self.root / "record.json"

    def write(self, path, text):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def git(self, root, *args):
        return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()

    def init(self, root):
        self.git(root, "init", "-q")
        self.git(root, "config", "user.email", "fixture@example.invalid")
        self.git(root, "config", "user.name", "Gate fixture")
        self.commit(root)

    def commit(self, root):
        self.git(root, "add", ".")
        self.git(root, "commit", "-qm", "fixture")

    def snapshot(self):
        return {str(p.relative_to(self.games)): hashlib.sha256(p.read_bytes()).hexdigest()
                for p in self.games.rglob("*") if p.is_file()}

    def gate(self, command="boundary", code=0, options=(), arguments=(), engine=None):
        result = subprocess.run([sys.executable, str((engine or self.worktree) / "scripts/game_gate.py"),
                                 "--record", str(self.record), *options, "sample", command, *arguments],
                                env=self.env, capture_output=True, text=True)
        self.assertEqual(result.returncode, code, result.stdout + result.stderr)
        return result

    def test_worktree_build_and_command_preserve_source(self):
        before = self.snapshot()
        self.gate("test")  # sibling engine returns 1; only the linked worktree returns 2
        record = json.loads(self.record.read_text())
        self.assertEqual(record["engine_revision"], self.git(self.worktree, "rev-parse", "HEAD"))
        self.assertEqual(record["game_revision"], self.git(self.games, "rev-parse", "HEAD"))
        self.assertFalse(record["taken_as_stands"])
        self.gate("echo", arguments=("--literal", "two words"))
        self.assertEqual(before, self.snapshot())  # includes Git index and source lockfile
        self.gate("test", engine=self.engine, code=7)
        self.assertEqual(json.loads(self.record.read_text())["build_exit_code"], 0)
        self.assertEqual(before, self.snapshot())
        (self.games / "sample/Cargo.lock").unlink()
        self.gate("test", options=("--allow-dirty",))
        self.assertFalse((self.games / "sample/Cargo.lock").exists())

    def test_override_missing_dirty_and_failure_codes(self):
        self.env["THRUM_GAMES"] = str(self.root / "missing")
        self.assertIn("missing games repository", self.gate(code=3).stderr)
        override = self.root / "elsewhere"
        self.games.rename(override)
        self.games = override
        self.env["THRUM_GAMES"] = str(override)
        self.gate()
        self.write(override / "sample/untracked.txt", "dirty snapshot")
        self.gate(code=4)
        before = self.snapshot()
        self.gate(options=("--allow-dirty",))
        record = json.loads(self.record.read_text())
        self.assertTrue(record["taken_as_stands"])
        self.assertIn("untracked.txt", record["games_status"])
        self.assertEqual(len(record["source_snapshot_sha256"]), 64)
        self.assertEqual(before, self.snapshot())
        self.write(override / "sample/src/lib.rs", "invalid Rust!")
        self.gate("test", options=("--allow-dirty",), code=6)

    def test_boundary_uses_shared_reach_and_test_edge_rules(self):
        for dependency in ('demo={path="../../thrum/examples/demo"}',
                           'thrum-fixtures={path="../../thrum/crates/thrum-fixtures"}',
                           'renamed={package="thrum-test-support",path="../../thrum/crates/thrum-test-support"}'):
            with self.subTest(dependency=dependency):
                self.write(self.games / "sample/Cargo.toml", self.manifest + dependency + "\n")
                result = self.gate(code=5, options=("--allow-dirty",))
                self.assertIn("test-crate-edges" if "thrum-fixtures" in dependency or "support" in dependency else "package-reach", result.stderr)
        self.write(self.games / "sample/Cargo.toml", self.manifest)
        for source in ('#[path="../../thrum/crates/engine/src/lib.rs"] mod private;',
                       'const X: &str = include_str!("../../thrum/crates/engine/src/lib.rs");'):
            self.write(self.games / "sample/build.rs", source)
            self.assertIn("package-reach", self.gate(code=5, options=("--allow-dirty",)).stderr)
        (self.games / "sample/build.rs").unlink()
        for target in ('[lib]\npath="../../thrum/crates/engine/src/lib.rs"\n',
                       '[[bin]]\nname="outside"\npath="../../thrum/crates/engine/src/lib.rs"\n'):
            self.write(self.games / "sample/Cargo.toml", self.manifest + target)
            self.assertIn("package-reach", self.gate(code=5, options=("--allow-dirty",)).stderr)
        self.write(self.games / "sample/Cargo.toml", self.manifest + '[dev-dependencies]\nthrum-fixtures={path="../../thrum/crates/thrum-fixtures"}\n')
        self.gate(options=("--allow-dirty",))
        self.write(self.games / "Cargo.toml", '[workspace]\nmembers=["sample"]\n[workspace.dependencies]\nrenamed={package="thrum-fixtures",path="../thrum/crates/thrum-fixtures"}\n')
        self.write(self.games / "sample/Cargo.toml", self.manifest + 'renamed.workspace=true\n')
        self.assertIn("test-crate-edges", self.gate(code=5, options=("--allow-dirty",)).stderr)

    def test_external_symlink_is_refused_without_touching_target(self):
        before = self.snapshot()
        self.gate(code=2, options=("--record", str(self.games / "record.json")))
        self.assertEqual(before, self.snapshot())
        target = self.root / "outside.txt"
        target.write_text("unchanged")
        (self.games / "sample/link").symlink_to(target)
        self.assertIn("external symlink", self.gate(code=2, options=("--allow-dirty",)).stderr)
        self.assertEqual(target.read_text(), "unchanged")


if __name__ == "__main__":
    unittest.main()
