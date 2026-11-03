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
from hold_macos_tool_root_clone import (
    _names, _open_lock, _seal_fd, open_sealed_clone, prepare_sealed_clone,
)
from inventory_headless_tool_roots import inventory_root
from prepare_headless_cargo_home import cargo_reuse_key, root_inventory


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

    def test_held_four_roots_reuse_checks_only_source_selection(self):
        with fixture() as (cache_source, manifest, owner):
            base = owner.parent
            rust_source = base / "rust-source"
            (rust_source / "bin").mkdir(parents=True)
            (rust_source / "bin/cargo").write_bytes(b"selected cargo")
            (rust_source / "bin/rustc").write_bytes(b"selected rustc")
            rust_tree = inventory_root(rust_source)
            rust_root, _ = prepare_sealed_clone(rust_source, owner, rust_tree)
            native_roots = {}
            for name in ("linker", "sdk"):
                source_root = base / f"{name}-source"
                source_root.mkdir()
                (source_root / f"{name}.bin").write_bytes(name.encode())
                tree = inventory_root(source_root)
                native_roots[name] = (tree, prepare_sealed_clone(source_root, owner, tree)[0])
            source = base / "game"
            source.mkdir()
            (source / "Cargo.toml").write_text("[workspace]\n")
            (source / "Cargo.lock").write_text("version = 4\n")
            (source / "rust-toolchain.toml").write_text(
                '[toolchain]\nchannel = "1.99.1"\n',
            )
            (source / "game.rs").write_text("fn a() {}\n")
            tools = {
                name: {"path": str(rust_root / "bin" / name),
                       "sha256": hashlib.sha256((rust_root / "bin" / name).read_bytes()).hexdigest(),
                       "bytes": (rust_root / "bin" / name).stat().st_size}
                for name in ("cargo", "rustc")
            }
            target = "aarch64-apple-darwin"
            reuse = cargo_reuse_key(source, target, tools)
            manifest.write_text(json.dumps({
                "format": 2, "privateRoot": root_inventory(cache_source),
                "target": target, "toolExecutables": tools, "reuseKey": reuse,
            }, sort_keys=True, separators=(",", ":")) + "\n")
            _, tree, legacy, manifest_sha = cache.prepare_sealed_cache(
                cache_source, manifest, owner,
            )
            with (cache.open_sealed_cache(owner, tree, legacy, manifest_sha) as cache_lease,
                  open_sealed_clone(owner, rust_tree) as rust_lease,
                  open_sealed_clone(owner, native_roots["linker"][0]) as linker_lease,
                  open_sealed_clone(owner, native_roots["sdk"][0]) as sdk_lease):
                self.assertEqual(linker_lease.root, native_roots["linker"][1])
                self.assertEqual(sdk_lease.root, native_roots["sdk"][1])
                selected = cache.HeldNativeCacheSelection(
                    cache_lease, rust_lease, linker_lease, sdk_lease,
                )
                for field, value in (("path", "/wrong/cargo"), ("sha256", "0" * 64)):
                    changed = json.loads(cache_lease.manifest_bytes)
                    changed["toolExecutables"]["cargo"][field] = value
                    with patch.object(cache_lease, "manifest_bytes", json.dumps(changed).encode()):
                        with self.assertRaisesRegex(ValueError, "held cargo differs"):
                            cache.HeldNativeCacheSelection(
                                cache_lease, rust_lease, linker_lease, sdk_lease,
                            )
                with patch.object(cache_lease, "manifest_bytes", b"[]"):
                    with self.assertRaisesRegex(ValueError, "incompatible dependency selection"):
                        cache.HeldNativeCacheSelection(
                            cache_lease, rust_lease, linker_lease, sdk_lease,
                        )
                for lease, name in ((linker_lease, "linker"), (sdk_lease, "sdk")):
                    with patch.object(lease, "assert_live", side_effect=ValueError(f"lost {name}")):
                        with self.assertRaisesRegex(ValueError, f"lost {name}"):
                            selected.verify_reuse(source, target)
                with (patch.object(cache, "root_inventory", side_effect=AssertionError("cache scan")),
                      patch.object(cache, "sha256_file", side_effect=AssertionError("tool hash"))):
                    self.assertEqual(selected.verify_reuse(source, target), reuse)
                    with self.assertRaisesRegex(ValueError, "target differs"):
                        selected.verify_reuse(source, "x86_64-apple-darwin")
                    (source / "game.rs").write_text("fn b() {}\n")
                    self.assertEqual(selected.verify_reuse(source, target), reuse)
                    (source / "Cargo.toml").write_text("[workspace]\nresolver = '3'\n")
                    with self.assertRaisesRegex(ValueError, "cannot be reused"):
                        selected.verify_reuse(source, target)
            with self.assertRaisesRegex(ValueError, "closed"):
                selected.verify_reuse(source, target)


if __name__ == "__main__":
    unittest.main()
