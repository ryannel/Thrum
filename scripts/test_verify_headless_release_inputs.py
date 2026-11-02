"""Exact-input and refusal checks for the first native release selector."""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

from inventory_headless_tool_roots import inventory_roots
from prepare_headless_cargo_home import cargo_reuse_key, root_inventory
from verify_headless_release_inputs import (
    TARGET, PROFILE, bounded_regular_bytes, verify_release_inputs,
)


def write_json(path, value):
    path.write_text(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n")


def origin_record():
    return {"format": 1, "sources": {
        "rustToolchain": {"url": "https://static.rust-lang.org/dist/", "version": "1.99.1"},
        "linkerToolchain": {"url": "https://developer.apple.com/download/", "version": "27.2"},
        "sdk": {"url": "https://developer.apple.com/download/", "version": "27.2"},
        "cargoRegistry": {"url": "https://index.crates.io/", "version": "lock fixture"},
    }}


@unittest.skipUnless(os.name == "posix", "first native verifier requires POSIX")
class VerifyHeadlessReleaseInputs(unittest.TestCase):
    def test_manifest_file_swap_cannot_read_outside_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            selected = root / "native.json"
            selected.write_bytes(b"inside")
            outside = root / "outside"
            outside.write_bytes(b"outside")
            real_open = os.open
            swapped = False

            def swap_file(path, flags, *args, **kwargs):
                nonlocal swapped
                if path == selected.name and kwargs.get("dir_fd") is not None and not swapped:
                    swapped = True
                    selected.unlink()
                    selected.symlink_to(outside)
                return real_open(path, flags, *args, **kwargs)

            with mock.patch("verify_headless_release_inputs.os.open", side_effect=swap_file):
                with self.assertRaises((OSError, ValueError)):
                    bounded_regular_bytes(selected, 64)
            self.assertTrue(swapped)

    def test_unicode_install_path_matches_canonical_candidate_encoding(self):
        with tempfile.TemporaryDirectory(prefix="thrum-é-") as directory:
            args, _, _ = self.fixture(Path(directory).resolve())
            self.assertTrue(verify_release_inputs(*args)["matchedInputRoots"])

    def fixture(self, root):
        source = root / "source"
        source.mkdir()
        (source / "Cargo.toml").write_text("[workspace]\n")
        (source / "Cargo.lock").write_text("version = 4\n")
        (source / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "1.99.1"\n')
        (source / "game.rs").write_text("fn a() {}\n")
        private_home = root / "private-home"
        private_home.mkdir()
        (private_home / "dependency.crate").write_bytes(b"dependency")
        rust_toolchain = root / "rust-toolchain"
        (rust_toolchain / "bin").mkdir(parents=True)
        cargo = rust_toolchain / "bin/cargo"
        rustc = rust_toolchain / "bin/rustc"
        cargo.write_bytes(b"cargo")
        rustc.write_bytes(b"rustc")
        linker_toolchain = root / "linker-toolchain"
        (linker_toolchain / "bin").mkdir(parents=True)
        linker = linker_toolchain / "bin/clang"
        linker.write_bytes(b"clang")
        sdk = root / "sdk"
        sdk.mkdir()
        (sdk / "header.h").write_bytes(b"sdk")
        tool_executables = {
            name: {"path": str(path.resolve()),
                   "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                   "bytes": path.stat().st_size}
            for name, path in (("cargo", cargo), ("rustc", rustc))
        }
        reuse_key = cargo_reuse_key(source, TARGET, tool_executables)
        cache_manifest = root / "candidate.json"
        candidate = {
            "format": 2, "target": TARGET,
            "lockSha256": hashlib.sha256((source / "Cargo.lock").read_bytes()).hexdigest(),
            "toolExecutables": tool_executables, "reuseKey": reuse_key,
            "privateRoot": root_inventory(private_home),
        }
        write_json(cache_manifest, candidate)
        release = root / "release"
        release.mkdir()
        evidence = release / "origin.json"
        write_json(evidence, origin_record())
        manifest_path = release / "native.json"
        manifest = {
            "format": 1, "profile": PROFILE, "target": TARGET,
            "selectionKey": reuse_key["sha256"],
            "lockSha256": candidate["lockSha256"],
            "dependencyRoot": candidate["privateRoot"],
            "toolRoots": inventory_roots({
                "rustToolchain": rust_toolchain,
                "linkerToolchain": linker_toolchain, "sdk": sdk,
            }),
            "originEvidence": {
                "file": evidence.name,
                "sha256": hashlib.sha256(evidence.read_bytes()).hexdigest(),
            },
        }
        write_json(manifest_path, manifest)
        arguments = (
            manifest_path, source, cache_manifest, private_home,
            rust_toolchain, linker_toolchain, sdk, cargo, rustc, linker, TARGET,
        )
        return arguments, manifest, evidence

    def test_matching_roots_and_code_edit_pass_but_changed_inputs_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            args, manifest, evidence = self.fixture(Path(directory).resolve())
            report = verify_release_inputs(*args)
            self.assertTrue(report["matchedInputRoots"])
            self.assertFalse(report["buildReady"])
            names = (
                "manifest", "source", "cache-manifest", "private-home",
                "rust-toolchain", "linker-toolchain", "sdk", "cargo", "rustc",
                "linker", "target",
            )
            command = [sys.executable, str(Path(__file__).with_name("verify_headless_release_inputs.py"))]
            for name, value in zip(names, args):
                command.extend((f"--{name}", str(value)))
            completed = subprocess.run(command, capture_output=True, check=True)
            self.assertEqual(json.loads(completed.stdout), report)
            (args[1] / "game.rs").write_text("fn b() {}\n")
            self.assertEqual(verify_release_inputs(*args), report)
            (args[3] / "dependency.crate").write_bytes(b"changed")
            with self.assertRaisesRegex(ValueError, "cache bytes changed"):
                verify_release_inputs(*args)
            (args[3] / "dependency.crate").write_bytes(b"dependency")
            (args[1] / "Cargo.toml").write_text("[workspace]\nresolver = '3'\n")
            with self.assertRaisesRegex(ValueError, "cannot be reused"):
                verify_release_inputs(*args)
            (args[1] / "Cargo.toml").write_text("[workspace]\n")
            evidence.write_bytes(b"changed origin")
            with self.assertRaisesRegex(ValueError, "origin evidence bytes differ"):
                verify_release_inputs(*args)
            write_json(evidence, origin_record())
            (args[6] / "header.h").write_bytes(b"changed SDK")
            with self.assertRaisesRegex(ValueError, "tool roots differ"):
                verify_release_inputs(*args)
            self.assertEqual(manifest["profile"], PROFILE)

    def test_unofficial_origin_refuses_even_when_its_hash_is_selected(self):
        with tempfile.TemporaryDirectory() as directory:
            args, manifest, evidence = self.fixture(Path(directory).resolve())
            record = origin_record()
            record["sources"]["rustToolchain"]["url"] = "https://untrusted.example/dist/"
            write_json(evidence, record)
            manifest["originEvidence"]["sha256"] = hashlib.sha256(evidence.read_bytes()).hexdigest()
            write_json(args[0], manifest)
            with self.assertRaisesRegex(ValueError, "not an official HTTPS origin"):
                verify_release_inputs(*args)

    def test_manifest_syntax_and_selection_fail_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            args, manifest, _ = self.fixture(Path(directory).resolve())
            args[0].write_text('{"format":1,"format":1}\n')
            with self.assertRaisesRegex(ValueError, "duplicate JSON field"):
                verify_release_inputs(*args)
            manifest["unexpected"] = True
            write_json(args[0], manifest)
            with self.assertRaisesRegex(ValueError, "unsupported or missing fields"):
                verify_release_inputs(*args)
            del manifest["unexpected"]
            manifest["selectionKey"] = "0" * 64
            write_json(args[0], manifest)
            with self.assertRaisesRegex(ValueError, "selection differs"):
                verify_release_inputs(*args)
            manifest["selectionKey"] = cargo_reuse_key(
                args[1], TARGET, json.loads(args[2].read_text())["toolExecutables"]
            )["sha256"]
            write_json(args[0], manifest)
            with self.assertRaisesRegex(ValueError, "caller target differs"):
                verify_release_inputs(*args[:-1], "x86_64-apple-darwin")
            candidate = json.loads(args[2].read_text())
            candidate["lockSha256"] = "0" * 64
            manifest["lockSha256"] = "0" * 64
            write_json(args[2], candidate)
            write_json(args[0], manifest)
            with self.assertRaisesRegex(ValueError, "lockfile bytes differ"):
                verify_release_inputs(*args)


if __name__ == "__main__":
    unittest.main()
