"""Small macOS fixtures for the unready held private Cargo cache."""

from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
import unittest
from unittest.mock import patch

if sys.platform == "darwin":
    import fcntl

import hold_macos_private_cargo_cache as cache
from hold_macos_tool_root_clone import _names, _open_lock, _seal_fd
from prepare_headless_cargo_home import root_inventory


@contextmanager
def fixture():
    with tempfile.TemporaryDirectory() as temporary:
        base = Path(temporary)
        source = base / "private-home"
        (source / "registry" / "cache").mkdir(parents=True)
        (source / "registry" / "cache" / "a.crate").write_bytes(b"archive")
        (source / "registry" / "index").mkdir()
        (source / "registry" / "index" / "record").write_bytes(b"index")
        manifest = base / "private-home.manifest.json"
        manifest.write_text(json.dumps({
            "format": 2, "privateRoot": root_inventory(source),
        }, sort_keys=True, separators=(",", ":")) + "\n")
        owner = base / "owner"
        owner.mkdir(mode=0o700)
        try:
            yield source, manifest, owner
        finally:
            # The test owns this entire temporary directory. Production
            # selections remain append-only and have no automatic unseal.
            for directory, directories, files in os.walk(owner):
                for name in [*directories, *files]:
                    path = Path(directory) / name
                    if not path.is_symlink():
                        os.chflags(path, os.stat(path).st_flags & ~stat.UF_IMMUTABLE)
                        path.chmod(0o700 if path.is_dir() else 0o600)
                os.chflags(directory, os.stat(directory).st_flags & ~stat.UF_IMMUTABLE)
                os.chmod(directory, 0o700)


@unittest.skipUnless(sys.platform == "darwin", "requires macOS clonefile and file flags")
class HeldMacosPrivateCargoCache(unittest.TestCase):
    def test_two_digest_domains_and_manifest_are_held(self):
        with fixture() as (source, manifest, owner):
            root, tree, legacy, manifest_sha = cache.prepare_sealed_cache(source, manifest, owner)
            self.assertEqual(manifest_sha, hashlib.sha256(manifest.read_bytes()).hexdigest())
            self.assertEqual(legacy, root_inventory(root))
            self.assertNotEqual(tree["sha256"], legacy["sha256"])
            with cache.open_sealed_cache(owner, tree, legacy, manifest_sha) as first:
                with cache.open_sealed_cache(owner, tree, legacy, manifest_sha) as second:
                    first.assert_live()
                    second.assert_live()
                    self.assertEqual(first.root, root)
                    self.assertEqual(second.root, root)
                    with self.assertRaises(BlockingIOError):
                        _open_lock(owner, _names(tree)[2], fcntl.LOCK_EX)
                    with self.assertRaises(PermissionError):
                        with (root / "registry" / "cache" / "a.crate").open("ab") as output:
                            output.write(b"unexpected")
            with self.assertRaisesRegex(ValueError, "closed"):
                first.assert_live()

    def test_extra_empty_directory_refuses_even_when_legacy_bytes_match(self):
        with fixture() as (source, manifest, owner):
            root, tree, legacy, manifest_sha = cache.prepare_sealed_cache(source, manifest, owner)
            os.chflags(root, os.stat(root).st_flags & ~stat.UF_IMMUTABLE)
            root.chmod(0o700)
            extra = root / "extra-empty"
            extra.mkdir()
            extra_fd = os.open(extra, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
            try:
                _seal_fd(extra_fd, True)
            finally:
                os.close(extra_fd)
            root_fd = os.open(root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
            try:
                _seal_fd(root_fd, True)
            finally:
                os.close(root_fd)
            self.assertEqual(root_inventory(root), legacy)
            with self.assertRaisesRegex(ValueError, "differ from selected inventory"):
                cache.open_sealed_cache(owner, tree, legacy, manifest_sha)

    def test_close_releases_root_lease_after_sidecar_close_error(self):
        with fixture() as (source, manifest, owner):
            _, tree, legacy, manifest_sha = cache.prepare_sealed_cache(source, manifest, owner)
            selection = cache.open_sealed_cache(owner, tree, legacy, manifest_sha)
            record_fd = selection._files[0][1]
            real_close = os.close

            def close_then_fail(fd):
                real_close(fd)
                if fd == record_fd:
                    raise OSError("injected close failure")

            with patch.object(cache.os, "close", side_effect=close_then_fail):
                with self.assertRaisesRegex(OSError, "injected close failure"):
                    selection.close()
            exclusive = _open_lock(owner, _names(tree)[2], fcntl.LOCK_EX)
            os.close(exclusive)

    def test_manifest_copy_drift_refuses_cold_open(self):
        with fixture() as (source, manifest, owner):
            _, tree, legacy, manifest_sha = cache.prepare_sealed_cache(source, manifest, owner)
            copied = cache._paths(owner, tree)[1]
            os.chflags(copied, os.stat(copied).st_flags & ~stat.UF_IMMUTABLE)
            copied.chmod(0o600)
            copied.write_bytes(b"{}\n")
            copied_fd = os.open(copied, os.O_RDONLY | os.O_NOFOLLOW)
            try:
                _seal_fd(copied_fd, False)
            finally:
                os.close(copied_fd)
            with self.assertRaisesRegex(ValueError, "manifest copy differs"):
                cache.open_sealed_cache(owner, tree, legacy, manifest_sha)

    def test_interrupted_selection_record_never_opens(self):
        with fixture() as (source, manifest, owner):
            original = cache._publish_sealed_file

            def interrupt_record(path, raw):
                if path.name.startswith(".cargo-selection-"):
                    raise RuntimeError("interrupted cache record")
                original(path, raw)

            with patch.object(cache, "_publish_sealed_file", side_effect=interrupt_record):
                with self.assertRaisesRegex(RuntimeError, "interrupted cache record"):
                    cache.prepare_sealed_cache(source, manifest, owner)
            tree = cache.inventory_root(source)
            legacy = root_inventory(source)
            manifest_sha = hashlib.sha256(manifest.read_bytes()).hexdigest()
            with self.assertRaises(FileNotFoundError):
                cache.open_sealed_cache(owner, tree, legacy, manifest_sha)
            with self.assertRaisesRegex(ValueError, "already exists or is incomplete"):
                cache.prepare_sealed_cache(source, manifest, owner)


if __name__ == "__main__":
    unittest.main()
