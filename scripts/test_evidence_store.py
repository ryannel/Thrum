"""Restore real retained history in clean scratch and exercise local worktree sharing.

The repository running this test is never changed, including by restore.
"""
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from urllib.parse import unquote, urlsplit

import evidence_store as store
import repo_check
import shape_rules


def command(root, *args):
    return store.git(root, *args).decode().strip()


class EvidenceStore(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory()
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name).resolve() / "repo"
        self.root.mkdir()
        command(self.root, "init", "-q")
        command(self.root, "config", "user.name", "Evidence fixture")
        command(self.root, "config", "user.email", "fixture@example.invalid")
        (self.root / "README.md").write_text("fixture\n")
        (self.root / ".gitignore").write_text("/docs/evidence/\n")
        command(self.root, "add", ".")
        command(self.root, "commit", "-qm", "fixture")

    def write(self, path, data):
        result = self.root / path
        result.parent.mkdir(parents=True, exist_ok=True)
        result.write_bytes(data)
        return result

    def test_local_shares_one_store_between_worktrees_and_reports_missing(self):
        first, second = (Path(self.scratch.name).resolve() / name for name in ("first", "second"))
        for path in (first, second):
            command(self.root, "worktree", "add", "--detach", str(path), "HEAD")
        self.assertEqual(store.store(first), store.store(second))
        self.assertEqual(store.store(first), self.root.parent / "thrum-evidence-store")
        source = self.write("new-capture", b"new uncommitted video\x00" * 200)
        logical = "docs/evidence/example/capture.mp4"
        with patch.dict(os.environ, {}, clear=True):
            store.put(first, source, logical)
            self.assertFalse((first / "docs/evidence").exists())
            self.assertEqual(command(first, "status", "--porcelain", "-uall"), f"?? {store.MANIFEST}")
            command(first, "add", store.MANIFEST)
            command(first, "commit", "-qm", "local manifest")
            command(second, "checkout", "--detach", command(first, "rev-parse", "HEAD"))
            store.restore(second, logical)
            self.assertEqual(hashlib.sha256((second / logical).read_bytes()).hexdigest(), hashlib.sha256(source.read_bytes()).hexdigest())
            self.assertEqual(command(second, "status", "--porcelain"), "")
            entry = store.load(second)["entries"][0]
            self.assertEqual(entry["kind"], "local")
            self.assertNotIn("blob", entry)
            self.assertNotIn("commit", entry)
            (store.store(second) / entry["sha256"]).write_bytes(b"corrupt")
            self.assertTrue(store.check(second))
            (store.store(second) / entry["sha256"]).unlink()
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                self.assertEqual(store.check(second), [])
            self.assertIn(logical, output.getvalue())
            self.assertIn("not on this machine", output.getvalue())
            with self.assertRaisesRegex(FileNotFoundError, "not on this machine"):
                store.restore(second, logical)
            with self.assertRaisesRegex(ValueError, "not listed"):
                store.restore(second, "docs/evidence/unknown.mp4")
            with self.assertRaises(ValueError):
                store.restore(second, "docs/evidence/../../outside")

    def test_size_rule_refuses_staged_capture_and_exempts_only_registered_reference(self):
        name = "docs/evidence/example/large.mp4"
        self.write(name, b"x" * (store.CAP + 1))
        command(self.root, "add", "-f", name)
        errors = shape_rules.check(self.root, "evidence-size", ["docs/evidence"])
        self.assertIn("evidence-size", "\n".join(errors))
        self.assertIn("evidence_store.py put", "\n".join(errors))
        self.assertTrue(store.check(self.root))
        self.write(".config/exact-baselines.json", json.dumps({"baselines": [{"reference": name}]}).encode())
        self.assertEqual(shape_rules.check(self.root, "evidence-size", ["docs/evidence"]), [])
        self.write(".config/exact-baselines.json", b'{"baselines": []}')
        self.write(name, b"x" * store.CAP)
        self.assertEqual(shape_rules.check(self.root, "evidence-size", ["docs/evidence"]), [])

    def test_archive_recover_and_link_checks_ignore_disk_presence(self):
        name = "docs/evidence/example/history.mp4"
        original = b"v" * (store.CAP + 1)
        self.write(name, original)
        page = self.write("review.md", f"[video]({name})\n".encode())
        command(self.root, "add", "-f", ".")
        command(self.root, "commit", "-qm", "history")
        store.archive(self.root)
        self.assertFalse((self.root / name).exists())
        self.assertEqual(store.check(self.root), [])
        self.assertEqual(repo_check.check_links(self.root, [page])[0], [])
        store.restore(self.root, name)
        self.assertEqual((self.root / name).read_bytes(), original)
        self.assertEqual(repo_check.check_links(self.root, [page])[0], [])
        (self.root / name).write_bytes(b"user modification")
        with self.assertRaisesRegex(ValueError, "different local bytes"):
            store.restore(self.root, name)
        page.write_text("[unlisted](docs/evidence/unlisted.mp4)\n")
        self.write("docs/evidence/unlisted.mp4", b"present but untracked")
        self.assertIn("missing/outside", "\n".join(repo_check.check_links(self.root, [page])[0]))
        manifest = store.load(self.root)
        manifest["entries"][0]["sha256"] = "0" * 64
        store.save(self.root, manifest)
        self.assertTrue(store.check(self.root))

    def test_restored_markdown_is_only_a_link_target(self):
        name = "docs/evidence/v1-workload/G7-CHROME-VALLEY-2026-11-07.md"
        original = store.git(store.ROOT, "show", f"77c70eaac:{name}")
        archived = self.write(name, original)
        page = self.write("review.md", f"[checks]({name}#checks)\n".encode())
        command(self.root, "add", "-f", name, page.name)
        command(self.root, "commit", "-qm", "original review")
        commit = command(self.root, "rev-parse", "HEAD")
        entry = {"kind": "history", "path": name, "size": len(original),
                 "sha256": hashlib.sha256(original).hexdigest(),
                 "blob": command(self.root, "rev-parse", f"HEAD:{name}"), "commit": commit}
        command(self.root, "rm", "-q", name)
        store.save(self.root, {"schema": 1, "entries": [entry]})

        self.assertFalse(archived.exists())
        self.assertEqual(repo_check.check_links(self.root, repo_check.markdown_files(self.root)), ([], 1))
        store.restore(self.root, name)
        self.assertEqual(archived.read_bytes(), original)
        files = repo_check.markdown_files(self.root)
        self.assertIn(archived, files)
        self.assertEqual(repo_check.check_links(self.root, files), ([], 1))
        page.write_text(page.read_text() + "[missing](missing.md)\n")
        self.assertEqual(repo_check.check_links(self.root, files),
                         (["missing/outside local link: review.md: missing.md"], 2))

    def test_one_manifest_under_config_keeps_relocated_paths_reserved(self):
        self.assertTrue(store.MANIFEST.startswith(".config/"))
        name = "docs/evidence/example/relocated.json"
        self.write(name, b"historical bytes")
        command(self.root, "add", "-f", name)
        command(self.root, "commit", "-qm", "relocated")
        commit = command(self.root, "rev-parse", "HEAD")
        entry = {"kind": "history", "path": name, "size": 16, "sha256": hashlib.sha256(b"historical bytes").hexdigest(),
                 "blob": command(self.root, "rev-parse", f"HEAD:{name}"), "commit": commit, "relocated": True}
        command(self.root, "rm", "-q", name)
        store.save(self.root, {"schema": 1, "entries": [entry]})
        self.assertEqual(store.check(self.root), [])
        source = self.write("scratch", b"new bytes")
        with self.assertRaisesRegex(ValueError, "reserved for historical evidence"):
            store.put(self.root, source, name)
        store.restore(self.root, name)
        self.assertEqual((self.root / name).read_bytes(), b"historical bytes")
        self.assertEqual(command(self.root, "status", "--porcelain", "--", "docs"), "D  " + name)
        store.save(self.root, {"schema": 1, "entries": [dict(entry, kind="local", relocated=True)]})
        with self.assertRaisesRegex(ValueError, "invalid relocated mark"):
            store.load(self.root)


if __name__ == "__main__":
    unittest.main(verbosity=2)
