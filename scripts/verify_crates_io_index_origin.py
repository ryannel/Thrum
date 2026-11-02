"""Compare selected Cargo sparse-index cache records with official HTTPS data.

This qualifies origin of records present in one private cache. It does not
prove that the cache has every upstream version or make a build ready.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path

from prepare_headless_cargo_home import index_key, root_inventory
from verify_crates_io_archive_origin import official_stream
from verify_rust_component_archive import (
    file_identity, ordinary_file_beneath, small_bytes,
)


MAX_INDEX = 8 * 1024 * 1024
MAX_TOTAL = 64 * 1024 * 1024
MAX_NAMES = 1000


def bounded_bytes(stream, maximum):
    raw = stream.read(maximum + 1)
    if len(raw) > maximum:
        raise ValueError("registry index record exceeds size bound")
    return raw


def cache_versions(raw, name):
    # Cargo 1.99.1 SummariesCache v3, index schema v2. The layout is
    # documented by Cargo's versioned source and refused on any format change.
    if not raw.startswith(b"\x03\x02\x00\x00\x00") or not raw.endswith(b"\0"):
        raise ValueError(f"unsupported Cargo sparse-cache format: {name}")
    fields = raw[5:].split(b"\0")
    if len(fields) < 4 or (len(fields) - 2) % 2:
        raise ValueError(f"malformed Cargo sparse-cache record: {name}")
    if not fields[0].startswith((b"etag: ", b"last-modified: ")) or len(fields[0]) > 1024:
        raise ValueError(f"unsupported Cargo sparse-cache version marker: {name}")
    records = {}
    for offset in range(1, len(fields) - 1, 2):
        version, line = fields[offset], fields[offset + 1]
        if not version or not line or version in records:
            raise ValueError(f"duplicate or empty cached version: {name}")
        entry = json.loads(line)
        if (not isinstance(entry, dict) or entry.get("name") != name
                or entry.get("vers") != version.decode("utf-8")):
            raise ValueError(f"cached index entry identity differs: {name}")
        records[version] = line
    if not records:
        raise ValueError(f"empty Cargo sparse-cache record: {name}")
    return records


def official_versions(raw, name):
    records = {}
    for line in raw.splitlines():
        entry = json.loads(line)
        if not isinstance(entry, dict) or entry.get("name") != name:
            raise ValueError(f"official index entry identity differs: {name}")
        version = entry.get("vers")
        if not isinstance(version, str) or not version:
            raise ValueError(f"duplicate or empty official index version: {name}")
        version_bytes = version.encode("utf-8")
        if version_bytes in records:
            raise ValueError(f"duplicate or empty official index version: {name}")
        records[version_bytes] = line
    if not records:
        raise ValueError(f"empty official index record: {name}")
    return records


def compare(candidate_path, private_home, fetch=official_stream):
    candidate = json.loads(small_bytes(candidate_path, 4 * 1024 * 1024))
    if candidate.get("format") != 2 or candidate.get("target") != "aarch64-apple-darwin":
        raise ValueError("unsupported private Cargo candidate")
    selected = candidate.get("registryIndexRecords")
    if not isinstance(selected, dict) or not selected or len(selected) > MAX_NAMES:
        raise ValueError("invalid selected index record set")
    if not private_home.is_absolute() or private_home.is_symlink() or not private_home.is_dir():
        raise ValueError("private Cargo home must be an ordinary absolute directory")
    if private_home.resolve(strict=True) != private_home:
        raise ValueError("private Cargo home path must be canonical")
    if root_inventory(private_home) != candidate.get("privateRoot"):
        raise ValueError("private Cargo home differs from selected candidate")
    index_roots = list((private_home / "registry/index").iterdir())
    if (len(index_roots) != 1 or index_roots[0].is_symlink()
            or not index_roots[0].is_dir()
            or not index_roots[0].name.startswith("index.crates.io-")):
        raise ValueError("expected one private crates.io sparse index")
    index_relative = Path("registry/index") / index_roots[0].name
    before = private_home.stat(follow_symlinks=False)
    root_fd = os.open(private_home, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        if file_identity(os.fstat(root_fd)) != file_identity(before):
            raise ValueError("private Cargo home changed before index comparison")
        checked_versions = 0
        total = 0
        for name in sorted(selected):
            key = index_key(name)
            expected = selected[name]
            if (not isinstance(expected, dict) or set(expected) != {"sha256", "bytes"}
                    or not isinstance(expected["sha256"], str)
                    or len(expected["sha256"]) != 64
                    or any(c not in "0123456789abcdef" for c in expected["sha256"])
                    or not isinstance(expected["bytes"], int)
                    or isinstance(expected["bytes"], bool)
                    or not 0 < expected["bytes"] <= MAX_INDEX):
                raise ValueError(f"invalid selected index identity: {name}")
            total += expected["bytes"]
            if total > MAX_TOTAL:
                raise ValueError("selected index records exceed total size bound")
            with ordinary_file_beneath(root_fd, index_relative / ".cache" / key,
                                       MAX_INDEX) as local:
                info = os.fstat(local.fileno())
                cached = bounded_bytes(local, expected["bytes"])
                if file_identity(os.fstat(local.fileno())) != file_identity(info):
                    raise ValueError(f"private index record changed: {name}")
            if {"sha256": hashlib.sha256(cached).hexdigest(), "bytes": len(cached)} != expected:
                raise ValueError(f"private index record differs from candidate: {name}")
            cached_versions = cache_versions(cached, name)
            url = "https://index.crates.io/" + key.as_posix()
            with fetch(url) as remote:
                official = official_versions(bounded_bytes(remote, MAX_INDEX), name)
            for version, line in cached_versions.items():
                if official.get(version) != line:
                    raise ValueError(f"cached index entry differs from official: {name} {version!r}")
            checked_versions += len(cached_versions)
        with ordinary_file_beneath(root_fd, index_relative / "config.json", 4096) as config:
            local_config = bounded_bytes(config, 4096)
        with fetch("https://index.crates.io/config.json") as remote:
            official_config = bounded_bytes(remote, 4096)
        if local_config != official_config:
            raise ValueError("private registry config differs from official index")
        if file_identity(private_home.stat(follow_symlinks=False)) != file_identity(before):
            raise ValueError("private Cargo home changed during index comparison")
    finally:
        os.close(root_fd)
    if root_inventory(private_home) != candidate["privateRoot"]:
        raise ValueError("private Cargo home changed during index comparison")
    return {"format": 1, "target": candidate["target"],
            "privateRootSha256": candidate["privateRoot"]["sha256"],
            "indexNames": len(selected), "cachedVersionsMatched": checked_versions,
            "buildReady": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--private-home", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(compare(args.candidate, args.private_home),
                     sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()
