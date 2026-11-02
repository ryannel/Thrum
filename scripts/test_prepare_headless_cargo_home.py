"""Failure-boundary checks for first-profile private Cargo cache preparation."""

from pathlib import Path
import os
import sys
import tempfile
import time
import unittest

from prepare_headless_cargo_home import collect_bounded, possible_headless_packages, prepare


class PrepareHeadlessCargoHome(unittest.TestCase):
    def test_selected_package_walk_ignores_dev_and_refuses_build_hooks(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory)
            manifest = source / "examples/i1-mobile-game/Cargo.toml"
            manifest.parent.mkdir(parents=True)
            manifest.write_text("[package]\nname = 'i1-mobile-game'\n")
            packages = [
                {"id": name, "name": name, "version": "0.1.0", "source": None,
                 "manifest_path": str(source / path / "Cargo.toml"),
                 "targets": [{"kind": ["lib"]}]}
                for name, path in (
                    ("i1-mobile-game", "examples/i1-mobile-game"),
                    ("core", "crates/core"), ("build-helper", "crates/build-helper"),
                    ("dev-only", "crates/dev-only"),
                )
            ]
            metadata = {"packages": packages, "resolve": {"nodes": [
                {"id": "i1-mobile-game", "features": [], "deps": [
                    {"pkg": "core", "dep_kinds": [{"kind": None}]},
                    {"pkg": "build-helper", "dep_kinds": [{"kind": "build"}]},
                    {"pkg": "dev-only", "dep_kinds": [{"kind": "dev"}]},
                ]},
                *({"id": name, "features": [], "deps": []}
                  for name in ("core", "build-helper", "dev-only")),
            ]}}
            selected = possible_headless_packages(metadata, source)
            self.assertEqual({package["name"] for package in selected},
                             {"i1-mobile-game", "core", "build-helper"})
            packages[1]["targets"] = [{"kind": ["custom-build"]}]
            with self.assertRaisesRegex(ValueError, "executable dependency hooks"):
                possible_headless_packages(metadata, source)

    def test_metadata_output_is_stopped_during_collection(self):
        with tempfile.TemporaryDirectory() as directory:
            started = time.monotonic()
            with self.assertRaisesRegex(ValueError, "output limit"):
                collect_bounded(
                    [sys.executable, "-c", "import sys,time;sys.stdout.buffer.write(b'x'*2048);sys.stdout.flush();time.sleep(10)"],
                    Path(directory), os.environ.copy(), stdout_limit=1024,
                )
            self.assertLess(time.monotonic() - started, 2)

    @unittest.skipUnless(os.name == "posix", "process groups require POSIX")
    def test_metadata_descendant_cannot_hold_output_pipes(self):
        with tempfile.TemporaryDirectory() as directory:
            started = time.monotonic()
            with self.assertRaisesRegex(ValueError, "descendants retained output pipes"):
                collect_bounded(
                    [sys.executable, "-c", (
                        "import subprocess,sys;"
                        "subprocess.Popen([sys.executable,'-c','import time;time.sleep(10)'])"
                    )],
                    Path(directory), os.environ.copy(),
                )
            self.assertLess(time.monotonic() - started, 3)

    def fixture(self, root):
        source = root / "accepted-input"
        source.mkdir()
        shared = root / "shared-home"
        shared.mkdir()
        tools = root / "tools"
        tools.mkdir()
        cargo = tools / "cargo"
        rustc = tools / "rustc"
        cargo.write_bytes(b"fixture")
        rustc.write_bytes(b"fixture")
        return source, shared, cargo, rustc, root / "private-home"

    def test_bad_lock_archive_refuses_and_removes_only_new_cache(self):
        with tempfile.TemporaryDirectory() as directory:
            source, shared, cargo, rustc, destination = self.fixture(Path(directory))
            (source / "Cargo.lock").write_text(
                'version = 4\n[[package]]\nname = "hashbrown"\nversion = "0.17.1"\n'
                'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
                'checksum = "' + ("0" * 64) + '"\n'
            )
            registry = "index.crates.io-fixture"
            index = shared / "registry/index" / registry
            (index / ".cache/ha/sh").mkdir(parents=True)
            (index / "config.json").write_bytes(b"{}")
            (index / ".cache/ha/sh/hashbrown").write_bytes(b"index fixture")
            archive = shared / "registry/cache" / registry
            archive.mkdir(parents=True)
            (archive / "hashbrown-0.17.1.crate").write_bytes(b"wrong archive")

            with self.assertRaisesRegex(ValueError, "checksum mismatch"):
                prepare(source, shared, destination, cargo, rustc, "aarch64-apple-darwin")
            self.assertFalse(destination.exists())
            self.assertFalse(destination.with_name("private-home.manifest.json").exists())
            self.assertTrue((source / "Cargo.lock").is_file())

    def test_ambient_cargo_config_refuses_before_creating_cache(self):
        with tempfile.TemporaryDirectory() as directory:
            source, shared, cargo, rustc, destination = self.fixture(Path(directory))
            config = source / ".cargo/config.toml"
            config.parent.mkdir()
            config.write_text('[build]\nrustc-workspace-wrapper = "/usr/bin/false"\n')

            with self.assertRaisesRegex(ValueError, "ambient Cargo configuration"):
                prepare(source, shared, destination, cargo, rustc, "aarch64-apple-darwin")
            self.assertTrue(config.is_file())
            self.assertFalse(destination.exists())

    def test_lock_version_cannot_escape_cache_path(self):
        with tempfile.TemporaryDirectory() as directory:
            source, shared, cargo, rustc, destination = self.fixture(Path(directory))
            (source / "Cargo.lock").write_text(
                'version = 4\n[[package]]\nname = "hashbrown"\n'
                'version = "../outside"\n'
                'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
                'checksum = "' + ("0" * 64) + '"\n'
            )
            with self.assertRaisesRegex(ValueError, "unsupported registry package spelling"):
                prepare(source, shared, destination, cargo, rustc, "aarch64-apple-darwin")
            self.assertFalse(destination.exists())
            self.assertFalse((Path(directory) / "outside.crate").exists())


if __name__ == "__main__":
    unittest.main()
