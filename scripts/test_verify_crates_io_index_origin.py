"""Boundary fixtures for Cargo sparse-index origin comparison."""

import hashlib
import io
import json
import os
from pathlib import Path
import tempfile
import unittest

from prepare_headless_cargo_home import root_inventory
from verify_crates_io_index_origin import cache_versions, compare, official_versions


@unittest.skipUnless(os.name == "posix", "private Cargo index inventory requires POSIX")
class CratesIoIndexOrigin(unittest.TestCase):
    def fixture(self, root):
        private = root / "private"
        index = private / "registry/index/index.crates.io-fixture"
        record = index / ".cache/sa/mp/sample"
        record.parent.mkdir(parents=True)
        line = b'{"name":"sample","vers":"1.0.0","cksum":"abc"}'
        cached = b'\x03\x02\x00\x00\x00etag: "fixture"\x001.0.0\x00' + line + b'\x00'
        record.write_bytes(cached)
        config = b'{"dl":"https://static.crates.io/crates","api":"https://crates.io"}\n'
        (index / "config.json").write_bytes(config)
        candidate = root / "candidate.json"
        candidate.write_text(json.dumps({
            "format": 2, "target": "aarch64-apple-darwin",
            "registryIndexRecords": {"sample": {
                "sha256": hashlib.sha256(cached).hexdigest(), "bytes": len(cached),
            }},
            "privateRoot": root_inventory(private),
        }))

        def fetch(url):
            if url.endswith("config.json"):
                return io.BytesIO(config)
            return io.BytesIO(line + b"\n")

        return candidate, private, record, line, fetch

    def test_cached_version_matches_official_record(self):
        with tempfile.TemporaryDirectory() as directory:
            candidate, private, _, _, fetch = self.fixture(Path(directory).resolve())
            report = compare(candidate, private, fetch)
            self.assertEqual((report["indexNames"], report["cachedVersionsMatched"]), (1, 1))
            self.assertFalse(report["buildReady"])

    def test_changed_official_record_and_private_cache_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            candidate, private, record, line, fetch = self.fixture(Path(directory).resolve())
            def changed(url):
                if url.endswith("config.json"):
                    return fetch(url)
                return io.BytesIO(line.replace(b'"abc"', b'"bad"') + b"\n")
            with self.assertRaisesRegex(ValueError, "differs from official"):
                compare(candidate, private, changed)
            record.write_bytes(record.read_bytes().replace(b'"abc"', b'"bad"'))
            with self.assertRaisesRegex(ValueError, "private Cargo home differs"):
                compare(candidate, private, fetch)

    def test_malformed_cache_and_official_identity_refuse(self):
        line = b'{"name":"other","vers":"1.0.0"}'
        raw = b'\x03\x02\x00\x00\x00etag: "x"\x001.0.0\x00' + line + b'\x00'
        with self.assertRaisesRegex(ValueError, "identity differs"):
            cache_versions(raw, "sample")
        with self.assertRaisesRegex(ValueError, "unsupported Cargo sparse-cache format"):
            cache_versions(b"not Cargo's versioned cache", "sample")
        valid = b'{"name":"sample","vers":"1.0.0","cksum":"abc"}'
        changed = b'{"name":"sample","vers":"1.0.0","cksum":"bad"}'
        with self.assertRaisesRegex(ValueError, "duplicate or empty official index version"):
            official_versions(valid + b"\n" + changed + b"\n", "sample")


if __name__ == "__main__":
    unittest.main()
