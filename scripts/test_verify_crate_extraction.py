"""Positive and refusal fixtures for selected crate extraction matching."""

import hashlib
import io
import json
import os
from pathlib import Path
import tarfile
import tempfile
import unittest

from prepare_headless_cargo_home import root_inventory
from verify_crate_extraction import compare


@unittest.skipUnless(os.name == "posix", "private crate extraction matching requires POSIX")
class CrateExtraction(unittest.TestCase):
    def refresh_root(self, candidate, private):
        payload = json.loads(candidate.read_text())
        payload["privateRoot"] = root_inventory(private)
        candidate.write_text(json.dumps(payload))

    def fixture(self, root, nested_empty=False, large_manifest=False):
        private = root / "private"
        registry = "index.crates.io-fixture"
        cache = private / "registry/cache" / registry
        source = private / "registry/src" / registry / "sample-1.2.3"
        cache.mkdir(parents=True)
        (source / "src").mkdir(parents=True)
        content = b"pub fn sample() {}\n"
        (source / "src/lib.rs").write_bytes(content)
        manifest_content = b"x" * (1024 * 1024 + 1) if large_manifest else None
        if manifest_content is not None:
            (source / "manifest.in").write_bytes(manifest_content)
        if nested_empty:
            (source / "empty/child").mkdir(parents=True)
        (source / ".cargo-ok").write_bytes(b'{"v":1}')
        archive = cache / "sample-1.2.3.crate"
        with tarfile.open(archive, "w:gz", format=tarfile.GNU_FORMAT) as tar:
            if nested_empty:
                member = tarfile.TarInfo("sample-1.2.3")
                member.type = tarfile.DIRTYPE
                tar.addfile(member)
            member = tarfile.TarInfo("sample-1.2.3/src/lib.rs")
            member.size = len(content)
            tar.addfile(member, io.BytesIO(content))
            if manifest_content is not None:
                member = tarfile.TarInfo("sample-1.2.3/manifest.in")
                member.size = len(manifest_content)
                tar.addfile(member, io.BytesIO(manifest_content))
            if nested_empty:
                member = tarfile.TarInfo("sample-1.2.3/empty/child")
                member.type = tarfile.DIRTYPE
                tar.addfile(member)
        archive_hash = hashlib.sha256(archive.read_bytes()).hexdigest()
        lockfile = root / "Cargo.lock"
        lockfile.write_text(
            'version = 4\n[[package]]\nname = "sample"\nversion = "1.2.3"\n'
            'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
            f'checksum = "{archive_hash}"\n'
        )
        candidate = root / "candidate.json"
        candidate.write_text(json.dumps({
            "format": 2, "target": "aarch64-apple-darwin",
            "lockSha256": hashlib.sha256(lockfile.read_bytes()).hexdigest(),
            "registryArchives": {archive.name: {
                "sha256": archive_hash, "bytes": archive.stat().st_size,
            }},
            "registryIndexRecords": {"sample": {"sha256": "0" * 64, "bytes": 1}},
            "privateRoot": root_inventory(private),
        }))
        return candidate, private, lockfile, source

    def test_archive_and_extracted_source_match(self):
        with tempfile.TemporaryDirectory() as directory:
            candidate, private, lockfile, _ = self.fixture(Path(directory).resolve())
            report = compare(candidate, private, lockfile)
            self.assertEqual(report["matchedFiles"], 1)
            self.assertEqual(report["matchedBytes"], len(b"pub fn sample() {}\n"))
            self.assertEqual(report["format"], 2)
            self.assertFalse(report["buildReady"])

    def test_nested_empty_directory_parents_match(self):
        with tempfile.TemporaryDirectory() as directory:
            candidate, private, lockfile, _ = self.fixture(
                Path(directory).resolve(), nested_empty=True,
            )
            report = compare(candidate, private, lockfile)
            self.assertEqual(report["matchedFiles"], 1)

    def test_crate_manifest_name_is_ordinary_payload(self):
        with tempfile.TemporaryDirectory() as directory:
            candidate, private, lockfile, _ = self.fixture(
                Path(directory).resolve(), large_manifest=True,
            )
            report = compare(candidate, private, lockfile)
            self.assertEqual(report["matchedFiles"], 2)

    def test_changed_or_extra_extracted_source_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            candidate, private, lockfile, source = self.fixture(Path(directory).resolve())
            (source / "src/lib.rs").write_bytes(b"altered\n")
            self.refresh_root(candidate, private)
            with self.assertRaisesRegex(ValueError, "extracted source differs from archive"):
                compare(candidate, private, lockfile)
            (source / "src/lib.rs").write_bytes(b"pub fn sample() {}\n")
            (source / "extra.rs").write_bytes(b"extra")
            self.refresh_root(candidate, private)
            with self.assertRaisesRegex(ValueError, "extracted source membership differs"):
                compare(candidate, private, lockfile)

    def test_unsupported_extraction_marker_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            candidate, private, lockfile, source = self.fixture(Path(directory).resolve())
            (source / ".cargo-ok").write_bytes(b"other")
            self.refresh_root(candidate, private)
            with self.assertRaisesRegex(ValueError, "unsupported Cargo extraction marker"):
                compare(candidate, private, lockfile)


if __name__ == "__main__":
    unittest.main()
