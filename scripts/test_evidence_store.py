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
        self.assertEqual(store.store(first), self.root.parent / "repo-evidence-store")
        source = self.write("new-capture", b"new uncommitted video\x00" * 200)
        logical = "docs/evidence/example/capture.mp4"
        with patch.dict(os.environ, {}, clear=True):
            store.put(first, source, logical)
            command(first, "add", store.MANIFEST, "docs/evidence/.gitignore")
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
        command(self.root, "add", name)
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
        command(self.root, "add", ".")
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

    def test_real_history_samples_in_clean_checkout(self):
        entries = [entry for entry in store.load(store.ROOT)["entries"] if entry["kind"] == "history"]
        self.assertTrue(entries, "the real migration must list history entries")
        largest = max(entries, key=lambda entry: entry["size"])
        video = next(entry for entry in entries if Path(entry["path"]).suffix in {".mp4", ".mov", ".mkv"})
        samples = {entry["path"]: entry for entry in [largest, video, *entries[::max(1, len(entries) // 5)]]}
        by_path = {entry["path"]: entry for entry in entries}
        review_links = []
        for name in sorted(store.tracked(store.ROOT)):
            if not name.startswith("docs/evidence/") or "review" not in name.lower() or not name.endswith(".md"):
                continue
            page = store.ROOT / name
            for match in repo_check.LINK.finditer(repo_check.prose(page.read_text())):
                url = urlsplit(match[1].split(' "', 1)[0].strip("<>"))
                if url.scheme or url.netloc or not url.path:
                    continue
                target = (page.parent / unquote(url.path)).resolve()
                if target.is_relative_to(store.ROOT):
                    relative = target.relative_to(store.ROOT).as_posix()
                    if relative in by_path and relative not in {link[1] for link in review_links}:
                        review_links.append((name, relative, match.group()))
                        samples[relative] = by_path[relative]
                if len(review_links) == 3:
                    break
            if len(review_links) == 3:
                break
        self.assertEqual(len(review_links), 3, "three original review links must be exercised")
        # A shared clone has the real history objects but no worktree files initially.
        clean = Path(self.scratch.name).resolve() / "clean"
        command(store.ROOT, "clone", "--quiet", "--shared", "--no-checkout", str(store.ROOT), str(clean))
        # A commit/index holding only the manifest makes restore's cleanliness observable,
        # even before the migration itself is committed.
        store.save(clean, {"schema": 1, "entries": list(samples.values())})
        command(clean, "add", store.MANIFEST, "docs/evidence/.gitignore")
        command(clean, "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "commit", "-qm", "clean restore fixture")
        for name, entry in samples.items():
            self.assertFalse((clean / name).exists())
            store.restore(clean, name)
            digest = hashlib.sha256((clean / name).read_bytes()).hexdigest()
            self.assertEqual(digest, entry["sha256"])
            print(f"independently hashed {name}: {digest}")
        self.assertEqual(command(clean, "status", "--porcelain"), "")
        for source, name, text in review_links:
            page = clean / source
            page.parent.mkdir(parents=True, exist_ok=True)
            page.write_text(text + "\n")
            command(clean, "add", source)
            self.assertEqual(repo_check.check_links(clean, [page])[0], [])
            (clean / name).unlink()
            self.assertEqual(repo_check.check_links(clean, [page])[0], [])
            store.restore(clean, name)
            self.assertEqual(hashlib.sha256((clean / name).read_bytes()).hexdigest(), by_path[name]["sha256"])
            print(f"original review link resolves absent/restored: {source}: {text}")


if __name__ == "__main__":
    unittest.main(verbosity=2)
