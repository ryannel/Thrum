"""Real scratch Git history: no remote, branch name or review-base parameter."""
import contextlib
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import shape_rules as shapes


class Ratchet(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name) / "repo"
        self.root.mkdir()
        self.git("init", "-q")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "user.name", "Shape fixture")
        self.write("crates/engine/Cargo.toml", '[package]\nname="engine"\n')
        self.write("crates/engine/src/lib.rs", 'pub fn first() -> Result<(), String> { todo!() }')
        self.assertEqual(self.tighten(), [])
        self.commit()

    def git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.root), *args], text=True, stderr=subprocess.STDOUT)

    def write(self, relative, text):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def commit(self):
        self.git("add", ".")
        self.git("commit", "-qm", "fixture")

    def tighten(self):
        with contextlib.redirect_stdout(io.StringIO()):
            return shapes.check(self.root, tighten=True)

    def check(self, root=None):
        with contextlib.redirect_stdout(io.StringIO()):
            return shapes.check(root or self.root)

    def inflate(self):
        self.write("crates/engine/src/new.rs", '// new source\npub fn second() -> Result<(), String> { todo!() }')
        current = shapes.inventory(self.root)[0]
        shapes.save_baseline(self.root / shapes.BASELINE, current)

    def test_uncommitted_committed_second_worktree_and_recovery(self):
        self.write("crates/engine/src/new.rs", 'pub fn second() -> Result<(), String> { todo!() }')
        self.assertTrue(self.check())
        self.assertTrue(self.tighten())
        self.inflate()
        self.assertIn("crates/engine/src/new.rs:2: string-errors:", "\n".join(self.check()))
        self.commit()
        self.assertTrue(self.check())
        other = Path(self.directory.name) / "other"
        self.git("worktree", "add", "--detach", str(other), "HEAD")
        self.assertTrue(self.check(other))
        (self.root / "crates/engine/src/new.rs").unlink()
        self.assertEqual(self.tighten(), [])
        self.assertEqual(self.check(), [])
        self.commit()
        # A committed reduction establishes a strictly lower mark.
        self.write("crates/engine/src/lib.rs", "")
        self.assertEqual(self.tighten(), [])
        self.commit()
        self.inflate()
        self.assertTrue(self.check())

    def test_move_and_split_carry_allowance(self):
        self.write("crates/engine/src/first.rs", (self.root / "crates/engine/src/lib.rs").read_text())
        self.write("crates/engine/src/lib.rs", "mod first;")
        self.assertEqual(self.tighten(), [])
        self.commit()
        self.assertEqual(self.check(), [])
        (self.root / "crates/engine/src/first.rs").rename(self.root / "crates/engine/src/moved.rs")
        self.assertEqual(self.tighten(), [])
        self.commit()
        self.assertEqual(self.check(), [])

    def test_moved_base_requires_first_registration_at_new_counts(self):
        # Main has no rule yet; the builder registers it on a private branch.
        (self.root / shapes.BASELINE).unlink()
        self.git("add", "-u")
        self.git("commit", "--amend", "--no-edit", "-q")
        self.git("branch", "-M", "main")
        self.git("checkout", "-qb", "builder")
        self.assertEqual(self.tighten(), [])
        self.commit()
        self.git("checkout", "main")
        self.write("crates/engine/src/new.rs", 'pub fn second() -> Result<(), String> { todo!() }')
        self.commit()
        self.git("checkout", "builder")
        self.git("rebase", "main")
        self.assertTrue(self.check())
        self.assertTrue(self.tighten())
        # Remake only unpublished builder history. Every replacement commit's
        # baseline equals its tree; no commit raises an already registered mark.
        self.git("reset", "--soft", "main")
        self.assertEqual(self.tighten(), [])
        self.commit()
        self.assertEqual(self.check(), [])
        revisions = self.git("rev-list", "main..HEAD").splitlines()
        self.assertEqual(len(revisions), 1)
        recorded = json.loads(self.git("show", f"{revisions[0]}:{shapes.BASELINE}"))
        self.assertEqual(recorded, shapes.inventory(self.root)[0])
        self.assertEqual(sum(recorded["rules"]["string-errors"]["files"].values()), 2)

    def test_new_rule_and_version_start_history(self):
        baseline = json.loads((self.root / shapes.BASELINE).read_text())
        del baseline["rules"]["path-attribute"]
        shapes.save_baseline(self.root / shapes.BASELINE, baseline)
        self.commit()
        current = shapes.inventory(self.root)[0]
        # A previously absent rule name starts at its registration count.
        current["rules"]["new-rule"] = {"version": 1, "files": {"fixture.rs": 5}}
        self.assertEqual(shapes.history_errors(self.root, current)[0], [])
        current["rules"]["string-errors"] = {"version": shapes.RULES["string-errors"][0] + 1, "files": {"fixture.rs": 9}}
        self.assertEqual(shapes.history_errors(self.root, current)[0], [])
        # Merely changing the baseline version cannot bypass the live matcher.
        shapes.save_baseline(self.root / shapes.BASELINE, current)
        self.assertTrue(self.check())

    def test_new_registration_and_matcher_version_pass_tree_check(self):
        baseline = json.loads((self.root / shapes.BASELINE).read_text())
        # Make a history in which this rule has not yet been registered.
        del baseline["rules"]["path-attribute"]
        shapes.save_baseline(self.root / shapes.BASELINE, baseline)
        self.git("add", ".")
        self.git("commit", "--amend", "--no-edit", "-q")
        self.write("crates/engine/src/lib.rs", '#[path="local.rs"] mod local;')
        self.assertEqual(self.tighten(), [])
        self.commit()
        self.assertEqual(self.check(), [])
        self.inflate()
        version, section, remedy = shapes.RULES["string-errors"]
        with patch.dict(shapes.RULES, {"string-errors": (version + 1, section, remedy)}):
            self.assertEqual(self.tighten(), [])
            self.commit()
            self.assertEqual(self.check(), [])


if __name__ == "__main__":
    unittest.main()
