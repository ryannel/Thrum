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
from release_cargo_selection import selection_sha256
from verify_headless_release_inputs import (
    TARGET, PROFILE, bounded_regular_bytes, verify_release_inputs,
)


def write_json(path, value):
    path.write_text(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n")


def origin_record(proofs):
    return {"format": 3, "sources": {
        "rustToolchain": {"url": "https://static.rust-lang.org/dist/", "version": "1.99.1"},
        "linkerToolchain": {"url": "https://developer.apple.com/download/", "version": "27.2"},
        "sdk": {"url": "https://developer.apple.com/download/", "version": "27.2"},
        "cargoRegistry": {"url": "https://index.crates.io/", "version": "lock fixture"},
    }, "proofs": proofs}


@unittest.skipUnless(os.name == "posix", "first native verifier requires POSIX")
class VerifyHeadlessReleaseInputs(unittest.TestCase):
    def rebind_proof(self, args, manifest, evidence, name, report):
        record = json.loads(evidence.read_text())
        path = evidence.parent / record["proofs"][name]["file"]
        write_json(path, report)
        record["proofs"][name]["sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
        write_json(evidence, record)
        manifest["originEvidence"]["sha256"] = hashlib.sha256(evidence.read_bytes()).hexdigest()
        write_json(args[0], manifest)

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
        archive_digest = hashlib.sha256(b"dependency").hexdigest()
        (source / "Cargo.lock").write_text(
            'version = 4\n[[package]]\nname = "sample"\nversion = "1.0.0"\n'
            'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
            f'checksum = "{archive_digest}"\n'
        )
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
        standard_library = rust_toolchain / "lib/rustlib/libstd.rlib"
        standard_library.parent.mkdir(parents=True)
        standard_library.write_bytes(b"standard library")
        rustlib = rust_toolchain / "lib/rustlib"
        (rustlib / "multirust-channel-manifest.toml").write_bytes(b"installed channel")
        (rustlib / "components").write_bytes(b"cargo\nrustc\nrust-std\n")
        for component in ("cargo", "rustc", "rust-std"):
            (rustlib / f"manifest-{component}-{TARGET}").write_bytes(
                f"file:{component}\n".encode(),
            )
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
            "registryArchives": {"sample-1.0.0.crate": {
                "sha256": archive_digest, "bytes": len(b"dependency"),
            }},
            "registryIndexRecords": {"sample": {
                "sha256": hashlib.sha256(b"index").hexdigest(), "bytes": len(b"index"),
            }},
        }
        write_json(cache_manifest, candidate)
        release = root / "release"
        release.mkdir()
        reports = {
            "cargoArchives": {
                "format": 2, "target": TARGET,
                "lockSha256": candidate["lockSha256"],
                "privateRootSha256": candidate["privateRoot"]["sha256"],
                "selectionSha256": selection_sha256(candidate),
                "archives": [{
                    "url": "https://static.crates.io/crates/sample/sample-1.0.0.crate",
                    "sha256": archive_digest, "bytes": len(b"dependency"),
                }], "buildReady": False,
            },
            "cargoIndex": {
                "format": 2, "target": TARGET,
                "privateRootSha256": candidate["privateRoot"]["sha256"],
                "selectionSha256": selection_sha256(candidate),
                "indexNames": 1, "cachedVersionsMatched": 1, "buildReady": False,
            },
            "cargoExtraction": {
                "format": 2, "target": TARGET,
                "privateRootSha256": candidate["privateRoot"]["sha256"],
                "selectionSha256": selection_sha256(candidate),
                "packages": 1, "matchedFiles": 1,
                "matchedBytes": len(b"dependency"), "buildReady": False,
            },
        }
        for name, component, file in (
            ("rustCargo", "cargo", cargo),
            ("rustc", "rustc", rustc),
            ("rustStd", "rust-std", standard_library),
        ):
            raw = file.read_bytes()
            metadata = [
                rustlib / "components",
                rustlib / f"manifest-{component}-{TARGET}",
                rustlib / "multirust-channel-manifest.toml",
            ]
            reports[name] = {
                "format": 3, "component": component, "target": TARGET,
                "channelSha256": "a" * 64,
                "archiveSha256": "b" * 64,
                "matchedFiles": 1, "matchedBytes": len(raw),
                "installedFiles": [{
                    "path": file.relative_to(rust_toolchain).as_posix(),
                    "sha256": hashlib.sha256(raw).hexdigest(), "bytes": len(raw),
                }],
                "installedMetadata": [{
                    "path": path.relative_to(rust_toolchain).as_posix(),
                    "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                    "bytes": path.stat().st_size,
                } for path in metadata],
                "buildReady": False,
            }
        proof_descriptors = {}
        for name, report in reports.items():
            path = release / f"{name}.json"
            write_json(path, report)
            proof_descriptors[name] = {
                "file": path.name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            }
        evidence = release / "origin.json"
        write_json(evidence, origin_record(proof_descriptors))
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
            self.assertTrue(report["matchedOriginProofs"])
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
            original_evidence = evidence.read_bytes()
            evidence.write_bytes(b"changed origin")
            with self.assertRaisesRegex(ValueError, "origin evidence bytes differ"):
                verify_release_inputs(*args)
            evidence.write_bytes(original_evidence)
            (args[6] / "header.h").write_bytes(b"changed SDK")
            with self.assertRaisesRegex(ValueError, "tool roots differ"):
                verify_release_inputs(*args)
            self.assertEqual(manifest["profile"], PROFILE)

    def test_unofficial_origin_refuses_even_when_its_hash_is_selected(self):
        with tempfile.TemporaryDirectory() as directory:
            args, manifest, evidence = self.fixture(Path(directory).resolve())
            record = json.loads(evidence.read_text())
            record["sources"]["rustToolchain"]["url"] = "https://untrusted.example/dist/"
            write_json(evidence, record)
            manifest["originEvidence"]["sha256"] = hashlib.sha256(evidence.read_bytes()).hexdigest()
            write_json(args[0], manifest)
            with self.assertRaisesRegex(ValueError, "not an official HTTPS origin"):
                verify_release_inputs(*args)

    def test_changed_or_rebound_origin_proof_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            args, manifest, evidence = self.fixture(Path(directory).resolve())
            descriptor = json.loads(evidence.read_text())["proofs"]["cargoArchives"]
            proof = evidence.parent / descriptor["file"]
            report = json.loads(proof.read_text())
            changed = dict(report)
            changed["archives"] = [dict(report["archives"][0], sha256="0" * 64)]
            write_json(proof, changed)
            with self.assertRaisesRegex(ValueError, "origin proof bytes differ"):
                verify_release_inputs(*args)
            report["privateRootSha256"] = "0" * 64
            self.rebind_proof(args, manifest, evidence, "cargoArchives", report)
            with self.assertRaisesRegex(ValueError, "origin proof selection differs"):
                verify_release_inputs(*args)
            report["privateRootSha256"] = manifest["dependencyRoot"]["sha256"]
            report["archives"][0]["url"] = "https://static.crates.io/crates/other/other-1.0.0.crate"
            self.rebind_proof(args, manifest, evidence, "cargoArchives", report)
            with self.assertRaisesRegex(ValueError, "cargo archive proof differs"):
                verify_release_inputs(*args)
            report["format"] = True
            self.rebind_proof(args, manifest, evidence, "cargoArchives", report)
            with self.assertRaisesRegex(ValueError, "origin proof selection differs"):
                verify_release_inputs(*args)

    def test_same_count_different_selection_receipt_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            args, manifest, evidence = self.fixture(Path(directory).resolve())
            candidate = json.loads(args[2].read_text())
            alternate = dict(candidate)
            alternate["registryIndexRecords"] = {"other": {
                "sha256": "0" * 64, "bytes": 5,
            }}
            self.assertEqual(len(alternate["registryIndexRecords"]),
                             len(candidate["registryIndexRecords"]))
            self.assertNotEqual(selection_sha256(alternate), selection_sha256(candidate))
            record = json.loads(evidence.read_text())
            proof = evidence.parent / record["proofs"]["cargoIndex"]["file"]
            report = json.loads(proof.read_text())
            report["selectionSha256"] = selection_sha256(alternate)
            self.rebind_proof(args, manifest, evidence, "cargoIndex", report)
            with self.assertRaisesRegex(ValueError, "origin proof selection differs"):
                verify_release_inputs(*args)

    def test_rust_receipt_does_not_match_selected_root(self):
        with tempfile.TemporaryDirectory() as directory:
            args, manifest, evidence = self.fixture(Path(directory).resolve())
            record = json.loads(evidence.read_text())
            proof = evidence.parent / record["proofs"]["rustc"]["file"]
            report = json.loads(proof.read_text())
            report["installedFiles"][0]["sha256"] = "0" * 64
            self.rebind_proof(args, manifest, evidence, "rustc", report)
            with self.assertRaisesRegex(ValueError, "receipts differ from selected tool root"):
                verify_release_inputs(*args)

    def test_rust_installer_metadata_receipt_does_not_match_selected_root(self):
        with tempfile.TemporaryDirectory() as directory:
            args, manifest, evidence = self.fixture(Path(directory).resolve())
            record = json.loads(evidence.read_text())
            proof = evidence.parent / record["proofs"]["rustc"]["file"]
            report = json.loads(proof.read_text())
            report["installedMetadata"][1]["sha256"] = "0" * 64
            self.rebind_proof(args, manifest, evidence, "rustc", report)
            with self.assertRaisesRegex(ValueError, "receipts differ from selected tool root"):
                verify_release_inputs(*args)

    def test_rust_shared_installer_metadata_must_agree(self):
        with tempfile.TemporaryDirectory() as directory:
            args, manifest, evidence = self.fixture(Path(directory).resolve())
            record = json.loads(evidence.read_text())
            proof = evidence.parent / record["proofs"]["rustc"]["file"]
            report = json.loads(proof.read_text())
            report["installedMetadata"][0]["sha256"] = "0" * 64
            self.rebind_proof(args, manifest, evidence, "rustc", report)
            with self.assertRaisesRegex(ValueError, "disagree on installed metadata"):
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
