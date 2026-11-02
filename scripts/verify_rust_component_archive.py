"""Compare a pinned Rust distribution archive with one installed component.

The caller supplies files fetched by the release-preparation job. Matching
archive and installed bytes is provenance evidence, not a reviewed release
manifest or a ready BuildRevision.
"""

import argparse
import hashlib
import json
import lzma
import os
from pathlib import Path, PurePosixPath
import re
import stat
import tomllib


VERSION = "1.99.1"
TARGET = "aarch64-apple-darwin"
CHANNEL = f"channel-rust-{VERSION}.toml"
HEX = re.compile(r"[0-9a-f]{64}\Z")
COMPONENT = re.compile(r"[a-z][a-z0-9-]*\Z")


class BoundedXZReader:
    """Feed tar bounded decompressed chunks with an XZ dictionary memory cap."""

    def __init__(self, compressed):
        self.compressed = compressed
        self.decoder = lzma.LZMADecompressor(format=lzma.FORMAT_XZ,
                                             memlimit=256 * 1024 * 1024)
        self.decompressed = 0
        self.checked_trailing = False

    def read(self, size):
        if size < 0 or size > 8 * 1024 * 1024:
            raise ValueError("tar parser requested an oversized read")
        output = bytearray()
        while len(output) < size and not self.decoder.eof:
            if self.decoder.needs_input:
                chunk = self.compressed.read(64 * 1024)
                if not chunk:
                    raise ValueError("truncated XZ archive")
            else:
                chunk = b""
            part = self.decoder.decompress(chunk, max_length=min(64 * 1024, size - len(output)))
            self.decompressed += len(part)
            if self.decompressed > 4 * 1024 * 1024 * 1024:
                raise ValueError("decompressed archive exceeds first-profile bound")
            output.extend(part)
        if self.decoder.eof and not self.checked_trailing:
            if self.decoder.unused_data or self.compressed.read(1):
                raise ValueError("trailing compressed data after XZ stream")
            self.checked_trailing = True
        return bytes(output)


def file_identity(info):
    return (info.st_dev, info.st_ino, info.st_mode, info.st_size,
            info.st_mtime_ns, info.st_ctime_ns)


def ordinary_file(path, maximum):
    if not path.is_absolute() or path.is_symlink():
        raise ValueError(f"expected an ordinary absolute file: {path}")
    before = path.stat(follow_symlinks=False)
    handle = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    stream = os.fdopen(handle, "rb")
    info = os.fstat(stream.fileno())
    if (file_identity(before) != file_identity(info)
            or not stat.S_ISREG(info.st_mode) or info.st_size > maximum):
        stream.close()
        raise ValueError(f"file changed, is not regular or exceeds {maximum} bytes: {path}")
    return stream


def open_beneath(root_fd, relative, final_flags):
    parts = safe_relative(relative.as_posix()).parts
    current = os.dup(root_fd)
    try:
        for part in parts[:-1]:
            next_fd = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                              dir_fd=current)
            os.close(current)
            current = next_fd
        return os.open(parts[-1], final_flags | os.O_NOFOLLOW, dir_fd=current)
    except OSError as error:
        raise ValueError(f"installed component path escapes toolchain or is unavailable: {relative}") from error
    finally:
        os.close(current)


def ordinary_file_beneath(root_fd, relative, maximum):
    handle = open_beneath(root_fd, relative, os.O_RDONLY | os.O_NONBLOCK)
    stream = os.fdopen(handle, "rb")
    info = os.fstat(stream.fileno())
    if not stat.S_ISREG(info.st_mode) or info.st_size > maximum:
        stream.close()
        raise ValueError(f"installed file is not regular or exceeds {maximum} bytes: {relative}")
    return stream


def small_bytes_beneath(root_fd, relative, maximum):
    with ordinary_file_beneath(root_fd, relative, maximum) as stream:
        info = os.fstat(stream.fileno())
        raw = stream.read(maximum + 1)
        if len(raw) > maximum or file_identity(os.fstat(stream.fileno())) != file_identity(info):
            raise ValueError(f"installed file changed or exceeded limit: {relative}")
    with ordinary_file_beneath(root_fd, relative, maximum) as reopened:
        if file_identity(os.fstat(reopened.fileno())) != file_identity(info):
            raise ValueError(f"installed file path changed: {relative}")
    return raw


def small_bytes(path, maximum):
    with ordinary_file(path, maximum) as stream:
        info = os.fstat(stream.fileno())
        raw = stream.read(maximum + 1)
        if len(raw) > maximum or file_identity(os.fstat(stream.fileno())) != file_identity(info):
            raise ValueError(f"file changed or exceeded limit: {path}")
    if file_identity(path.stat(follow_symlinks=False)) != file_identity(info):
        raise ValueError(f"file path changed after read: {path}")
    return raw


def sha256_stream(stream):
    return hashlib.file_digest(stream, "sha256").hexdigest()


def safe_relative(raw):
    if not raw or "\\" in raw:
        raise ValueError(f"invalid component path: {raw}")
    path = PurePosixPath(raw)
    if path.is_absolute() or any(part in ("", ".", "..") for part in raw.split("/")):
        raise ValueError(f"invalid component path: {raw}")
    return path


def read_exact(stream, size):
    result = bytearray()
    while len(result) < size:
        chunk = stream.read(size - len(result))
        if not chunk:
            raise ValueError("truncated Rust component archive")
        result.extend(chunk)
    return bytes(result)


def tar_number(raw):
    digits = raw.split(b"\0", 1)[0].strip(b" ")
    if not digits or any(digit not in b"01234567" for digit in digits):
        raise ValueError("unsupported tar numeric field")
    return int(digits, 8)


def tar_members(stream):
    """Yield bounded ustar members; reject PAX/GNU extensions before buffering."""
    count = 0
    while True:
        header = read_exact(stream, 512)
        if header == bytes(512):
            if read_exact(stream, 512) != bytes(512):
                raise ValueError("invalid tar end marker")
            while tail := stream.read(64 * 1024):
                if any(tail):
                    raise ValueError("nonzero bytes after tar end marker")
            return
        count += 1
        if count > 100000:
            raise ValueError("archive has too many members")
        expected_checksum = tar_number(header[148:156])
        actual_checksum = sum(header[:148]) + 8 * ord(" ") + sum(header[156:])
        if expected_checksum != actual_checksum or header[257:263] not in (b"ustar\0", b"ustar "):
            raise ValueError("invalid or unsupported ustar header")
        kind = header[156:157]
        if kind not in (b"0", b"\0", b"5"):
            raise ValueError("unsupported tar member type")
        name = header[:100].split(b"\0", 1)[0].decode("utf-8")
        prefix = header[345:500].split(b"\0", 1)[0].decode("utf-8")
        path = safe_relative(f"{prefix}/{name}" if prefix else name)
        size = tar_number(header[124:136])
        if kind == b"5":
            if size:
                raise ValueError("tar directory contains data")
            yield path, None, 0, None
            continue
        if size > 1024 * 1024 * 1024:
            raise ValueError("archive member exceeds first-profile bound")
        content = hashlib.sha256()
        manifest_bytes = bytearray() if path.name == "manifest.in" else None
        if manifest_bytes is not None and size > 1024 * 1024:
            raise ValueError("archive component manifest is too large")
        remaining = size
        while remaining:
            chunk = read_exact(stream, min(64 * 1024, remaining))
            content.update(chunk)
            if manifest_bytes is not None:
                manifest_bytes.extend(chunk)
            remaining -= len(chunk)
        padding = (-size) % 512
        if padding and read_exact(stream, padding) != bytes(padding):
            raise ValueError("nonzero tar member padding")
        yield path, content.hexdigest(), size, bytes(manifest_bytes) if manifest_bytes is not None else None


def component_manifest(raw):
    records = {}
    for line in raw.decode("utf-8").splitlines():
        kind, separator, name = line.partition(":")
        if not separator or kind not in ("file", "dir"):
            raise ValueError("unsupported Rust component manifest entry")
        path = safe_relative(name)
        if path in records:
            raise ValueError(f"duplicate Rust component path: {name}")
        records[path] = kind
    if not records or "file" not in records.values():
        raise ValueError("Rust component manifest has no files")
    return records


def compare_component(channel_path, sidecar_path, archive_path, toolchain, component):
    """Return a candidate byte-match report; origin review remains external."""
    if not COMPONENT.fullmatch(component):
        raise ValueError("invalid Rust component name")
    if not toolchain.is_absolute() or toolchain.is_symlink() or not toolchain.is_dir():
        raise ValueError("toolchain must be an ordinary absolute directory")
    if toolchain.resolve(strict=True) != toolchain:
        raise ValueError("toolchain path must be canonical")
    before = toolchain.stat(follow_symlinks=False)
    root_fd = os.open(toolchain, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        if file_identity(os.fstat(root_fd)) != file_identity(before):
            raise ValueError("toolchain root changed before comparison")
        result = _compare_component(channel_path, sidecar_path, archive_path,
                                    root_fd, component)
        if file_identity(toolchain.stat(follow_symlinks=False)) != file_identity(before):
            raise ValueError("toolchain root changed during comparison")
        return result
    finally:
        os.close(root_fd)


def _compare_component(channel_path, sidecar_path, archive_path, root_fd, component):
    channel_raw = small_bytes(channel_path, 4 * 1024 * 1024)
    sidecar = small_bytes(sidecar_path, 128)
    expected_sidecar = f"{hashlib.sha256(channel_raw).hexdigest()}  {CHANNEL}\n".encode()
    if sidecar != expected_sidecar:
        raise ValueError("official channel manifest sidecar mismatch")
    channel = tomllib.loads(channel_raw.decode("utf-8"))
    installed_channel = tomllib.loads(small_bytes_beneath(
        root_fd, Path("lib/rustlib/multirust-channel-manifest.toml"), 4 * 1024 * 1024,
    ).decode("utf-8"))
    selected = channel["pkg"][component]["target"][TARGET]
    installed_selected = installed_channel["pkg"][component]["target"][TARGET]
    url = selected["xz_url"]
    expected_hash = selected["xz_hash"]
    if (not isinstance(url, str)
            or not url.startswith(f"https://static.rust-lang.org/dist/2026-10-15/")
            or not isinstance(expected_hash, str) or not HEX.fullmatch(expected_hash)):
        raise ValueError("unsupported official Rust component origin")
    if url != installed_selected["xz_url"] or expected_hash != installed_selected["xz_hash"]:
        raise ValueError("installed Rust component selection differs from channel")
    if f"{component}-{TARGET}" not in small_bytes_beneath(
        root_fd, Path("lib/rustlib/components"), 4096,
    ).decode("utf-8").splitlines():
        raise ValueError("Rust component is not installed")
    installed_manifest = small_bytes_beneath(
        root_fd, Path("lib/rustlib") / f"manifest-{component}-{TARGET}", 1024 * 1024,
    )
    records = component_manifest(installed_manifest)

    with ordinary_file(archive_path, 1024 * 1024 * 1024) as archive:
        before = os.fstat(archive.fileno())
        if sha256_stream(archive) != expected_hash:
            raise ValueError("Rust component archive hash differs from channel")
        archive.seek(0)
        hashes = {}
        sizes = {}
        manifests = {}
        unpacked_bytes = 0
        for name, digest, size, manifest_bytes in tar_members(BoundedXZReader(archive)):
            if digest is None:
                continue
            if name in hashes or name in manifests:
                raise ValueError(f"unsupported or duplicate archive member: {name}")
            unpacked_bytes += size
            if unpacked_bytes > 4 * 1024 * 1024 * 1024:
                raise ValueError("archive payload exceeds first-profile bounds")
            if manifest_bytes is not None:
                if manifests:
                    raise ValueError("archive has more than one component manifest")
                manifests[name] = manifest_bytes
            else:
                hashes[name] = digest
                sizes[name] = size
        archive.seek(0)
        if (sha256_stream(archive) != expected_hash
                or file_identity(os.fstat(archive.fileno())) != file_identity(before)
                or file_identity(archive_path.stat(follow_symlinks=False)) != file_identity(before)):
            raise ValueError("Rust component archive changed during comparison")

    if len(manifests) != 1:
        raise ValueError("expected one archive component manifest")
    manifest_name, archive_manifest = next(iter(manifests.items()))
    if archive_manifest != installed_manifest:
        raise ValueError("installed component manifest differs from archive")
    prefix = manifest_name.parent
    expected = {prefix / path for path, kind in records.items() if kind == "file"}
    actual = {path for path in hashes if path.parts[:len(prefix.parts)] == prefix.parts}
    if expected != actual:
        raise ValueError("archive payload membership differs from component manifest")

    matched_bytes = 0
    matched_files = 0
    for relative, kind in records.items():
        if kind == "dir":
            handle = open_beneath(root_fd, Path(*relative.parts),
                                  os.O_RDONLY | os.O_DIRECTORY)
            os.close(handle)
            continue
        path = Path(*relative.parts)
        with ordinary_file_beneath(root_fd, path, 1024 * 1024 * 1024) as stream:
            info = os.fstat(stream.fileno())
            if info.st_size != sizes[prefix / relative] or sha256_stream(stream) != hashes[prefix / relative]:
                raise ValueError(f"installed component file differs: {relative}")
            if file_identity(os.fstat(stream.fileno())) != file_identity(info):
                raise ValueError(f"installed component file changed: {relative}")
        with ordinary_file_beneath(root_fd, path, 1024 * 1024 * 1024) as reopened:
            if file_identity(os.fstat(reopened.fileno())) != file_identity(info):
                raise ValueError(f"installed component path changed: {relative}")
        matched_files += 1
        matched_bytes += info.st_size
    return {
        "format": 1,
        "component": component,
        "target": TARGET,
        "channelSha256": hashlib.sha256(channel_raw).hexdigest(),
        "archiveSha256": expected_hash,
        "matchedFiles": matched_files,
        "matchedBytes": matched_bytes,
        "buildReady": False,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("channel-manifest", "sidecar", "archive", "toolchain", "component"):
        parser.add_argument(f"--{name}", required=True)
    args = parser.parse_args()
    report = compare_component(
        Path(args.channel_manifest), Path(args.sidecar), Path(args.archive),
        Path(args.toolchain), args.component,
    )
    print(json.dumps(report, sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()
