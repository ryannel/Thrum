"""Selected-record retention and cross-owner refusal fixtures for macOS."""

import hashlib
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
import unittest
from unittest.mock import patch

import hold_macos_release_records as release_records
import hold_macos_private_cargo_cache as cargo_cache
from hold_macos_release_records import (
    open_sealed_release_records, prepare_sealed_release_records,
)
from hold_macos_tool_root_clone import open_sealed_clone, prepare_sealed_clone
from prepare_headless_cargo_home import cargo_reuse_key
import test_verify_headless_release_inputs as release_fixture
from verify_headless_release_inputs import TARGET


def clear_fixture_flags(owner):
    for directory, directories, files in os.walk(owner):
        for name in [*directories, *files]:
            path = Path(directory) / name
            if not path.is_symlink():
                os.chflags(path, os.stat(path).st_flags & ~stat.UF_IMMUTABLE)
                path.chmod(0o700 if path.is_dir() else 0o600)
        os.chflags(directory, os.stat(directory).st_flags & ~stat.UF_IMMUTABLE)
        Path(directory).chmod(0o700)


@unittest.skipUnless(sys.platform == "darwin", "requires macOS clonefile and file flags")
class HoldMacosReleaseRecords(unittest.TestCase):
    def test_four_root_binding_uses_witnesses_and_rejects_dependency_drift(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary).resolve()
            args, manifest, _ = release_fixture.VerifyHeadlessReleaseInputs().fixture(base)
            owner = base / "owner"
            owner.mkdir(mode=0o700)
            try:
                cloned = {}
                for name, path in (
                    ("rustToolchain", args[4]), ("linkerToolchain", args[5]),
                    ("sdk", args[6]),
                ):
                    tree = manifest["toolRoots"][name]
                    cloned[name], _ = prepare_sealed_clone(path, owner, tree)
                candidate = json.loads(args[2].read_text())
                for name in ("cargo", "rustc"):
                    candidate["toolExecutables"][name]["path"] = str(
                        cloned["rustToolchain"] / "bin" / name,
                    )
                candidate["reuseKey"] = cargo_reuse_key(
                    args[1], TARGET, candidate["toolExecutables"],
                )
                args[2].write_text(json.dumps(candidate, sort_keys=True, separators=(",", ":")) + "\n")
                manifest["selectionKey"] = candidate["reuseKey"]["sha256"]
                args[0].write_text(json.dumps(manifest, sort_keys=True, separators=(",", ":")) + "\n")
                cache_root, cache_tree, legacy, cache_sha = cargo_cache.prepare_sealed_cache(
                    args[3], args[2], owner,
                )
                manifest_sha = hashlib.sha256(args[0].read_bytes()).hexdigest()
                _, record_tree = prepare_sealed_release_records(
                    args[0], manifest_sha, args[1], args[2], TARGET, owner,
                )
                sealed_cache_manifest = cargo_cache._paths(owner, cache_tree)[1]
                with (cargo_cache.open_sealed_cache(owner, cache_tree, legacy, cache_sha) as cache,
                      open_sealed_release_records(
                          owner, record_tree, manifest_sha, args[1], sealed_cache_manifest,
                          TARGET,
                      ) as records,
                      open_sealed_clone(
                          owner, manifest["toolRoots"]["rustToolchain"],
                          records.rust_witnesses,
                      ) as rust,
                      open_sealed_clone(
                          owner, manifest["toolRoots"]["linkerToolchain"],
                      ) as linker,
                      open_sealed_clone(owner, manifest["toolRoots"]["sdk"]) as sdk):
                    self.assertEqual(cache.root, cache_root)
                    alternate = json.loads(cache.manifest_bytes)
                    alternate["format"] = 2.0
                    self.assertEqual(alternate, records.candidate)
                    alternate_bytes = (
                        json.dumps(alternate, sort_keys=True, separators=(",", ":")) + "\n"
                    ).encode()
                    with patch.object(cache, "manifest_bytes", alternate_bytes):
                        with self.assertRaisesRegex(ValueError, "cache manifest differs"):
                            cargo_cache.HeldNativeReleaseSelection(
                                records, cache, rust, linker, sdk,
                            )
                    with patch.object(cargo_cache, "sha256_file", side_effect=AssertionError("duplicate tool hash")):
                        selected = cargo_cache.HeldNativeReleaseSelection(
                            records, cache, rust, linker, sdk,
                        )
                        self.assertEqual(
                            selected.verify_reuse(args[1], TARGET)["sha256"],
                            manifest["selectionKey"],
                        )
                    (args[1] / "game.rs").write_text("fn changed() {}\n")
                    self.assertEqual(
                        selected.verify_reuse(args[1], TARGET)["sha256"],
                        manifest["selectionKey"],
                    )
                    (args[1] / "Cargo.toml").write_text("[workspace]\n# changed\n")
                    with self.assertRaisesRegex(ValueError, "cannot be reused"):
                        selected.verify_reuse(args[1], TARGET)
            finally:
                clear_fixture_flags(owner)

    def test_retained_records_and_rust_witness_bind_to_one_selection(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary).resolve()
            args, manifest, evidence = release_fixture.VerifyHeadlessReleaseInputs().fixture(base)
            owner = base / "owner"
            owner.mkdir(mode=0o700)
            try:
                manifest_sha = hashlib.sha256(args[0].read_bytes()).hexdigest()
                root, tree = prepare_sealed_release_records(
                    args[0], manifest_sha, args[1], args[2], TARGET, owner,
                )
                rust_tree = manifest["toolRoots"]["rustToolchain"]
                rust_root, _ = prepare_sealed_clone(args[4], owner, rust_tree)
                evidence.write_text("changed after selection\n")
                with open_sealed_release_records(
                    owner, tree, manifest_sha, args[1], args[2], TARGET,
                ) as selected:
                    self.assertEqual(selected.root, root)
                    self.assertEqual(selected.manifest_sha256, manifest_sha)
                    self.assertEqual(selected.manifest, manifest)
                    self.assertEqual(selected.candidate["privateRoot"], manifest["dependencyRoot"])
                    with open_sealed_clone(
                        owner, rust_tree, selected.rust_witnesses,
                    ) as rust:
                        self.assertEqual(rust.root, rust_root)
                        self.assertEqual(rust.witnesses, selected.rust_witnesses)
                    selected.assert_live()
                selected = open_sealed_release_records(
                    owner, tree, manifest_sha, args[1], args[2], TARGET,
                )
                fds = [fd for _, fd in selected._files]
                real_close = os.close

                def close_then_fail(fd):
                    real_close(fd)
                    if fd == fds[0]:
                        raise OSError("injected record close failure")

                with patch.object(release_records.os, "close", side_effect=close_then_fail):
                    with self.assertRaisesRegex(OSError, "injected record close failure"):
                        selected.close()
                for fd in fds:
                    with self.assertRaises(OSError):
                        os.fstat(fd)
                self.assertIsNone(selected._lease._root_fd)
                with self.assertRaisesRegex(ValueError, "selected reviewed digest"):
                    open_sealed_release_records(owner, tree, "0" * 64, args[1], args[2], TARGET)
                cache = json.loads(args[2].read_text())
                cache["privateRoot"]["sha256"] = "0" * 64
                args[2].write_text(json.dumps(cache, sort_keys=True, separators=(",", ":")) + "\n")
                with self.assertRaisesRegex(ValueError, "origin proof selection differs"):
                    open_sealed_release_records(owner, tree, manifest_sha, args[1], args[2], TARGET)
            finally:
                clear_fixture_flags(owner)


if __name__ == "__main__":
    unittest.main()
