"""Compare selected private Cargo source trees with their locked crate archives.

This is cold input-preparation evidence, not a compiler read trace or a ready
BuildRevision. The private cache must already match a selected candidate.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import stat
import zlib

from prepare_headless_cargo_home import root_inventory
from verify_crates_io_archive_origin import locked_archives
from verify_rust_component_archive import (
    file_identity, ordinary_file_beneath, sha256_stream,
    small_bytes, tar_members,
)


MAX_ARCHIVE = 64 * 1024 * 1024
MAX_DECOMPRESSED = 256 * 1024 * 1024
MAX_TOTAL = 1024 * 1024 * 1024
MAX_PACKAGES = 1000


class BoundedGzipReader:
    """Feed the strict tar parser bounded chunks from one gzip member."""

    def __init__(self, compressed):
        self.compressed = compressed
        self.decoder = zlib.decompressobj(wbits=31)
        self.pending = b""
        self.decompressed = 0
        self.checked_trailing = False

    def read(self, size):
        if size < 0 or size > 8 * 1024 * 1024:
            raise ValueError("tar parser requested an oversized read")
        output = bytearray()
        while len(output) < size and not self.decoder.eof:
            if not self.pending:
                self.pending = self.compressed.read(64 * 1024)
                if not self.pending:
                    raise ValueError("truncated gzip crate archive")
            part = self.decoder.decompress(
                self.pending, max_length=min(64 * 1024, size - len(output)),
            )
            self.pending = self.decoder.unconsumed_tail
            self.decompressed += len(part)
            if self.decompressed > MAX_DECOMPRESSED:
                raise ValueError("decompressed crate archive exceeds bound")
            output.extend(part)
        if self.decoder.eof and not self.checked_trailing:
            if self.decoder.unused_data or self.pending or self.compressed.read(1):
                raise ValueError("trailing compressed data after gzip crate")
            self.checked_trailing = True
        return bytes(output)


def source_members(root):
    files = set()
    directories = set()
    for path in root.rglob("*"):
        relative = path.relative_to(root).as_posix()
        kind = path.lstat().st_mode
        if stat.S_ISDIR(kind):
            directories.add(relative)
        elif stat.S_ISREG(kind):
            files.add(relative)
        else:
            raise ValueError(f"extracted source contains unsupported entry: {relative}")
    return files, directories


def compare(candidate_path, private_home, lockfile_path):
    candidate = json.loads(small_bytes(candidate_path, 4 * 1024 * 1024))
    if candidate.get("format") != 2 or candidate.get("target") != "aarch64-apple-darwin":
        raise ValueError("unsupported private Cargo candidate")
    lock_raw = small_bytes(lockfile_path, 4 * 1024 * 1024)
    if hashlib.sha256(lock_raw).hexdigest() != candidate.get("lockSha256"):
        raise ValueError("candidate lockfile differs from selected lockfile")
    locked = locked_archives(lock_raw)
    selected = candidate.get("registryArchives")
    if not isinstance(selected, dict) or not selected or len(selected) > MAX_PACKAGES:
        raise ValueError("invalid selected archive set")
    if not private_home.is_absolute() or private_home.is_symlink() or not private_home.is_dir():
        raise ValueError("private Cargo home must be an ordinary absolute directory")
    if private_home.resolve(strict=True) != private_home:
        raise ValueError("private Cargo home path must be canonical")
    if root_inventory(private_home) != candidate.get("privateRoot"):
        raise ValueError("private Cargo home differs from selected candidate")
    cache_roots = list((private_home / "registry/cache").iterdir())
    src_roots = list((private_home / "registry/src").iterdir())
    if (len(cache_roots) != 1 or len(src_roots) != 1
            or cache_roots[0].name != src_roots[0].name
            or cache_roots[0].is_symlink() or src_roots[0].is_symlink()
            or not cache_roots[0].name.startswith("index.crates.io-")):
        raise ValueError("expected matching private crates.io archive/source roots")
    registry_name = cache_roots[0].name
    before = private_home.stat(follow_symlinks=False)
    root_fd = os.open(private_home, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        if file_identity(os.fstat(root_fd)) != file_identity(before):
            raise ValueError("private Cargo home changed before extraction comparison")
        matched_files = 0
        matched_bytes = 0
        archive_total = 0
        for filename in sorted(selected):
            if filename not in locked:
                raise ValueError(f"selected archive absent from lockfile: {filename}")
            name, version, checksum = locked[filename]
            expected_identity = selected[filename]
            if (not isinstance(expected_identity, dict)
                    or set(expected_identity) != {"sha256", "bytes"}
                    or expected_identity["sha256"] != checksum
                    or not isinstance(expected_identity["bytes"], int)
                    or isinstance(expected_identity["bytes"], bool)
                    or not 0 < expected_identity["bytes"] <= MAX_ARCHIVE):
                raise ValueError(f"invalid selected archive identity: {filename}")
            archive_total += expected_identity["bytes"]
            if archive_total > MAX_TOTAL:
                raise ValueError("selected archives exceed total size bound")
            stem = f"{name}-{version}"
            archive_relative = Path("registry/cache") / registry_name / filename
            source_relative = Path("registry/src") / registry_name / stem
            source_root = private_home / source_relative
            if source_root.is_symlink() or not source_root.is_dir():
                raise ValueError(f"selected extracted source is missing: {stem}")
            archived = {}
            directories = set()
            seen_package_root = False
            with ordinary_file_beneath(root_fd, archive_relative, MAX_ARCHIVE) as archive:
                info = os.fstat(archive.fileno())
                if info.st_size != expected_identity["bytes"] or sha256_stream(archive) != checksum:
                    raise ValueError(f"private crate archive differs from lockfile: {filename}")
                archive.seek(0)
                for path, digest, size, _ in tar_members(BoundedGzipReader(archive)):
                    if path.parts == (stem,) and digest is None:
                        if seen_package_root:
                            raise ValueError(f"duplicate crate package-root directory: {stem}")
                        seen_package_root = True
                        continue
                    if len(path.parts) < 2 or path.parts[0] != stem:
                        raise ValueError(f"crate archive path escapes package root: {path}")
                    relative = PurePosixPath(*path.parts[1:]).as_posix()
                    if relative == ".cargo-ok" or relative in archived or relative in directories:
                        raise ValueError(f"duplicate or reserved crate archive path: {relative}")
                    if digest is None:
                        directories.add(relative)
                    else:
                        archived[relative] = (digest, size)
                if file_identity(os.fstat(archive.fileno())) != file_identity(info):
                    raise ValueError(f"private crate archive changed during parse: {filename}")
            if not archived:
                raise ValueError(f"crate archive contains no files: {filename}")
            file_paths, directory_paths = source_members(source_root)
            if file_paths != set(archived) | {".cargo-ok"}:
                raise ValueError(f"extracted source membership differs from archive: {stem}")
            for relative in (*archived, *tuple(directories)):
                parent = PurePosixPath(relative).parent
                while parent != PurePosixPath("."):
                    directories.add(parent.as_posix())
                    parent = parent.parent
            if directory_paths != directories:
                raise ValueError(f"extracted source directories differ from archive: {stem}")
            with ordinary_file_beneath(root_fd, source_relative / ".cargo-ok", 16) as marker:
                if marker.read(17) != b'{"v":1}':
                    raise ValueError(f"unsupported Cargo extraction marker: {stem}")
            for relative, (digest, size) in archived.items():
                with ordinary_file_beneath(root_fd, source_relative / relative, MAX_DECOMPRESSED) as file:
                    info = os.fstat(file.fileno())
                    if info.st_size != size or sha256_stream(file) != digest:
                        raise ValueError(f"extracted source differs from archive: {stem}/{relative}")
                    if file_identity(os.fstat(file.fileno())) != file_identity(info):
                        raise ValueError(f"extracted source changed during read: {stem}/{relative}")
                matched_files += 1
                matched_bytes += size
        if file_identity(private_home.stat(follow_symlinks=False)) != file_identity(before):
            raise ValueError("private Cargo home changed during extraction comparison")
    finally:
        os.close(root_fd)
    if root_inventory(private_home) != candidate["privateRoot"]:
        raise ValueError("private Cargo home changed during extraction comparison")
    return {"format": 1, "target": candidate["target"],
            "privateRootSha256": candidate["privateRoot"]["sha256"],
            "packages": len(selected), "matchedFiles": matched_files,
            "matchedBytes": matched_bytes, "buildReady": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--private-home", type=Path, required=True)
    parser.add_argument("--lockfile", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(compare(args.candidate, args.private_home, args.lockfile),
                     sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()
