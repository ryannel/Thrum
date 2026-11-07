"""Boundary fixtures for the cold crates.io archive origin comparison."""

import hashlib
import io
import json
import os
from pathlib import Path
import tempfile
import unittest

from prepare_headless_cargo_home import root_inventory
from verify_crates_io_archive_origin import compare


@unittest.skipUnless(os.name == "posix", "cold private Cargo origin comparison requires POSIX")
class CratesIoArchiveOrigin(unittest.TestCase):
    def fixture(self, root):
        content = b"selected crate archive"
        digest = hashlib.sha256(content).hexdigest()
        filename = "sample-1.2.3.crate"
        private = root / "private"
        cache = private / "registry/cache/index.crates.io-fixture"
        cache.mkdir(parents=True)
        (cache / filename).write_bytes(content)
        lockfile = root / "Cargo.lock"
        lockfile.write_text(
            'version = 4\n[[package]]\nname = "sample"\nversion = "1.2.3"\n'
            'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
            f'checksum = "{digest}"\n'
        )
        candidate = root / "candidate.json"
        candidate.write_text(json.dumps({
            "format": 2, "target": "aarch64-apple-darwin",
            "lockSha256": hashlib.sha256(lockfile.read_bytes()).hexdigest(),
            "privateRoot": root_inventory(private),
            "registryArchives": {filename: {"sha256": digest, "bytes": len(content)}},
            "registryIndexRecords": {"sample": {"sha256": "0" * 64, "bytes": 1}},
        }))
        return candidate, private, lockfile, content

    def test_official_and_private_archive_match_lockfile(self):
        with tempfile.TemporaryDirectory() as directory:
            candidate, private, lockfile, content = self.fixture(Path(directory).resolve())
            urls = []

            def fetch(url):
                urls.append(url)
                return io.BytesIO(content)

            report = compare(candidate, private, lockfile, fetch)
            self.assertEqual(urls, ["https://static.crates.io/crates/sample/sample-1.2.3.crate"])
            self.assertEqual(len(report["archives"]), 1)
            self.assertEqual(report["format"], 2)
            self.assertFalse(report["buildReady"])

    def test_changed_official_or_private_bytes_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            candidate, private, lockfile, content = self.fixture(Path(directory).resolve())
            with self.assertRaisesRegex(ValueError, "official archive differs"):
                compare(candidate, private, lockfile, lambda _: io.BytesIO(content[:-1] + b"x"))
            cache = private / "registry/cache/index.crates.io-fixture/sample-1.2.3.crate"
            cache.write_bytes(content + b"x")
            with self.assertRaisesRegex(ValueError, "private Cargo home differs"):
                compare(candidate, private, lockfile, lambda _: io.BytesIO(content))

    def test_selection_and_lockfile_mismatch_refuse_before_fetch(self):
        with tempfile.TemporaryDirectory() as directory:
            candidate, private, lockfile, content = self.fixture(Path(directory).resolve())
            lockfile.write_text(lockfile.read_text() + "\n")
            with self.assertRaisesRegex(ValueError, "candidate lockfile differs"):
                compare(candidate, private, lockfile, lambda _: io.BytesIO(content))
            lockfile.write_text(lockfile.read_text().rstrip() + "\n")
            selected = json.loads(candidate.read_text())
            selected["lockSha256"] = hashlib.sha256(lockfile.read_bytes()).hexdigest()
            selected["registryArchives"] = {"other-1.2.3.crate": next(iter(selected["registryArchives"].values()))}
            candidate.write_text(json.dumps(selected))
            with self.assertRaisesRegex(ValueError, "absent from lockfile"):
                compare(candidate, private, lockfile, lambda _: io.BytesIO(content))


if __name__ == "__main__":
    unittest.main()
