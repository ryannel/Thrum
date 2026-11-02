"""Positive and refusal fixtures for the first Rust component byte matcher."""

import hashlib
import io
import lzma
import os
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest import mock

import verify_rust_component_archive as verifier
from verify_rust_component_archive import compare_component


@unittest.skipUnless(os.name == "posix", "first native component verifier requires POSIX")
class RustComponentArchive(unittest.TestCase):
    def select_modified_archive(self, args, prior_hash):
        channel, sidecar, archive, toolchain, _ = args
        updated = channel.read_text().replace(
            prior_hash, hashlib.sha256(archive.read_bytes()).hexdigest(),
        )
        channel.write_text(updated)
        (toolchain / "lib/rustlib/multirust-channel-manifest.toml").write_text(updated)
        sidecar.write_text(f"{hashlib.sha256(channel.read_bytes()).hexdigest()}  {channel.name}\n")

    def fixture(self, root, extra=False, extra_manifest=False):
        toolchain = root / "toolchain"
        rustlib = toolchain / "lib/rustlib"
        rustlib.mkdir(parents=True)
        (toolchain / "bin").mkdir()
        (toolchain / "bin/cargo").write_bytes(b"selected cargo")
        (rustlib / "components").write_text("cargo-aarch64-apple-darwin\n")
        member_manifest = b"file:bin/cargo\n"
        (rustlib / "manifest-cargo-aarch64-apple-darwin").write_bytes(member_manifest)
        archive = root / "cargo.tar.xz"
        prefix = "cargo-1.99.1-aarch64-apple-darwin/cargo/"
        with tarfile.open(archive, "w:xz") as tar:
            for name, data in [
                (prefix + "bin/cargo", b"selected cargo"),
                (prefix + "manifest.in", member_manifest),
                *([(prefix + "extra", b"unexpected")] if extra else []),
                *([("other/manifest.in", member_manifest)] if extra_manifest else []),
            ]:
                item = tarfile.TarInfo(name)
                item.size = len(data)
                tar.addfile(item, io.BytesIO(data))
        archive_hash = hashlib.sha256(archive.read_bytes()).hexdigest()
        entry = (
            "[pkg.cargo.target.aarch64-apple-darwin]\n"
            'xz_url = "https://static.rust-lang.org/dist/2026-10-15/'
            'cargo-1.99.1-aarch64-apple-darwin.tar.xz"\n'
            f'xz_hash = "{archive_hash}"\n'
        )
        channel = root / "channel-rust-1.99.1.toml"
        sidecar = root / "channel-rust-1.99.1.toml.sha256"
        channel.write_text(entry)
        (rustlib / "multirust-channel-manifest.toml").write_text(entry)
        sidecar.write_text(f"{hashlib.sha256(channel.read_bytes()).hexdigest()}  {channel.name}\n")
        return channel, sidecar, archive, toolchain, "cargo"

    def test_verified_archive_matches_installed_component(self):
        with tempfile.TemporaryDirectory() as directory:
            args = self.fixture(Path(directory).resolve())
            report = compare_component(*args)
            self.assertEqual(report["matchedFiles"], 1)
            self.assertEqual(report["matchedBytes"], len(b"selected cargo"))
            self.assertFalse(report["buildReady"])

    def test_changed_installed_file_archive_and_sidecar_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            args = self.fixture(Path(directory).resolve())
            channel, sidecar, archive, toolchain, _ = args
            (toolchain / "bin/cargo").write_bytes(b"changed cargo")
            with self.assertRaisesRegex(ValueError, "installed component file differs"):
                compare_component(*args)
            (toolchain / "bin/cargo").write_bytes(b"selected cargo")
            archive.write_bytes(archive.read_bytes() + b"change")
            with self.assertRaisesRegex(ValueError, "archive hash differs"):
                compare_component(*args)
            other = Path(directory).resolve() / "sidecar-case"
            other.mkdir()
            args = self.fixture(other)
            channel, sidecar = args[:2]
            sidecar.write_text("0" * 64 + f"  {channel.name}\n")
            with self.assertRaisesRegex(ValueError, "sidecar mismatch"):
                compare_component(*args)

    def test_extra_archive_payload_refuses_even_with_matching_hash(self):
        with tempfile.TemporaryDirectory() as directory:
            args = self.fixture(Path(directory).resolve(), extra=True)
            with self.assertRaisesRegex(ValueError, "payload membership differs"):
                compare_component(*args)

    def test_second_component_manifest_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            args = self.fixture(Path(directory).resolve(), extra_manifest=True)
            with self.assertRaisesRegex(ValueError, "more than one component manifest"):
                compare_component(*args)

    def test_installed_parent_alias_cannot_reach_outside_toolchain(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            args = self.fixture(root)
            toolchain = args[3]
            outside = root / "outside"
            outside.mkdir()
            (outside / "cargo").write_bytes(b"selected cargo")
            (toolchain / "bin/cargo").unlink()
            (toolchain / "bin").rmdir()
            (toolchain / "bin").symlink_to(outside, target_is_directory=True)
            with self.assertRaisesRegex(ValueError, "escapes toolchain"):
                compare_component(*args)

    def test_parent_swap_after_root_open_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            args = self.fixture(root)
            toolchain = args[3]
            outside = root / "outside"
            outside.mkdir()
            (outside / "cargo").write_bytes(b"selected cargo")
            original = verifier.open_beneath
            swapped = False

            def swap_before_file(root_fd, relative, flags):
                nonlocal swapped
                if relative == Path("bin/cargo") and not swapped:
                    (toolchain / "bin").rename(toolchain / "old-bin")
                    (toolchain / "bin").symlink_to(outside, target_is_directory=True)
                    swapped = True
                return original(root_fd, relative, flags)

            with mock.patch.object(verifier, "open_beneath", side_effect=swap_before_file):
                with self.assertRaisesRegex(ValueError, "escapes toolchain"):
                    compare_component(*args)
            self.assertTrue(swapped)

    def test_pax_metadata_refuses_before_tar_buffering(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            args = self.fixture(root)
            archive = args[2]
            prior_hash = hashlib.sha256(archive.read_bytes()).hexdigest()
            name = "cargo-1.99.1-aarch64-apple-darwin/cargo/bin/cargo"
            with tarfile.open(archive, "w:xz", format=tarfile.PAX_FORMAT) as tar:
                item = tarfile.TarInfo(name)
                item.size = len(b"selected cargo")
                item.pax_headers = {"comment": "x" * (9 * 1024 * 1024)}
                tar.addfile(item, io.BytesIO(b"selected cargo"))
            self.select_modified_archive(args, prior_hash)
            with self.assertRaisesRegex(ValueError, "unsupported tar member type"):
                compare_component(*args)

    def test_concatenated_xz_stream_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            args = self.fixture(Path(directory).resolve())
            archive = args[2]
            prior_hash = hashlib.sha256(archive.read_bytes()).hexdigest()
            with archive.open("ab") as stream:
                stream.write(lzma.compress(b"unvalidated extra compressed stream"))
            self.select_modified_archive(args, prior_hash)
            with self.assertRaisesRegex(ValueError, "trailing compressed data"):
                compare_component(*args)

    def test_bounded_gnu_long_name_and_oversized_refusal(self):
        for name, accepted in (("component/" + "a" * 130, True),
                               ("componentx/" + "é" * 130, True),
                               ("component/" + "a" * 5000, False)):
            with self.subTest(length=len(name), multibyte="é" in name):
                tar_bytes = io.BytesIO()
                with tarfile.open(fileobj=tar_bytes, mode="w", format=tarfile.GNU_FORMAT) as tar:
                    item = tarfile.TarInfo(name)
                    item.size = 1
                    tar.addfile(item, io.BytesIO(b"x"))
                archive = lzma.compress(tar_bytes.getvalue())
                reader = verifier.BoundedXZReader(io.BytesIO(archive))
                if accepted:
                    members = list(verifier.tar_members(reader))
                    self.assertEqual(members[0][0].as_posix(), name)
                else:
                    with self.assertRaisesRegex(ValueError, "oversized GNU long-name"):
                        list(verifier.tar_members(reader))


if __name__ == "__main__":
    unittest.main()
