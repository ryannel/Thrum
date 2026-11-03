"""Compare a selected private Cargo cache with official crates.io archives.

This is cold release-preparation evidence. The caller supplies a previously
prepared cache and its candidate manifest; origin review and build readiness
remain separate.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import tomllib
import urllib.request

from prepare_headless_cargo_home import REGISTRY_SOURCE, SEMVER, index_key, root_inventory
from release_cargo_selection import selection_sha256
from verify_rust_component_archive import (
    file_identity, ordinary_file_beneath, small_bytes,
)


MAX_ARCHIVE = 64 * 1024 * 1024
MAX_TOTAL = 1024 * 1024 * 1024
MAX_PACKAGES = 1000
CHUNK = 64 * 1024


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, message, headers, new_url):
        raise ValueError(f"crate archive URL redirected: {request.full_url}")


def official_stream(url):
    request = urllib.request.Request(url, headers={"User-Agent": "Thrum-release-origin/1"})
    response = urllib.request.build_opener(NoRedirect).open(request, timeout=30)
    if response.status != 200 or response.geturl() != url:
        response.close()
        raise ValueError(f"unexpected official archive response: {url}")
    return response


def stream_identity(stream, maximum):
    digest = hashlib.sha256()
    size = 0
    while chunk := stream.read(min(CHUNK, maximum + 1 - size)):
        size += len(chunk)
        if size > maximum:
            raise ValueError("crate archive exceeds selected size bound")
        digest.update(chunk)
    return {"sha256": digest.hexdigest(), "bytes": size}


def locked_archives(raw):
    lock = tomllib.loads(raw.decode("utf-8"))
    if lock.get("version") != 4 or not isinstance(lock.get("package"), list):
        raise ValueError("unsupported Cargo lockfile")
    records = {}
    for package in lock["package"]:
        source = package.get("source")
        if source is None:
            continue
        if source != REGISTRY_SOURCE:
            raise ValueError("unsupported registry source in lockfile")
        name, version, checksum = package["name"], package["version"], package["checksum"]
        index_key(name)
        if (len(name) > 128 or len(version) > 128 or not SEMVER.fullmatch(version)
                or not isinstance(checksum, str) or len(checksum) != 64
                or any(char not in "0123456789abcdef" for char in checksum)):
            raise ValueError("invalid locked registry archive identity")
        filename = f"{name}-{version}.crate"
        if filename in records:
            raise ValueError("duplicate locked registry archive")
        records[filename] = (name, version, checksum)
    return records


def compare(candidate_path, private_home, lockfile_path, fetch=official_stream):
    candidate_raw = small_bytes(candidate_path, 4 * 1024 * 1024)
    candidate = json.loads(candidate_raw)
    if candidate.get("format") != 2 or candidate.get("target") != "aarch64-apple-darwin":
        raise ValueError("unsupported private Cargo candidate")
    lock_raw = small_bytes(lockfile_path, 4 * 1024 * 1024)
    if hashlib.sha256(lock_raw).hexdigest() != candidate.get("lockSha256"):
        raise ValueError("candidate lockfile differs from selected lockfile")
    locked = locked_archives(lock_raw)
    selected = candidate.get("registryArchives")
    if not isinstance(selected, dict) or not selected or len(selected) > MAX_PACKAGES:
        raise ValueError("invalid selected registry archives")
    if not private_home.is_absolute() or private_home.is_symlink() or not private_home.is_dir():
        raise ValueError("private Cargo home must be an ordinary absolute directory")
    if private_home.resolve(strict=True) != private_home:
        raise ValueError("private Cargo home path must be canonical")
    if root_inventory(private_home) != candidate.get("privateRoot"):
        raise ValueError("private Cargo home differs from selected candidate")
    cache_roots = list((private_home / "registry/cache").iterdir())
    if (len(cache_roots) != 1 or cache_roots[0].is_symlink()
            or not cache_roots[0].is_dir()
            or not cache_roots[0].name.startswith("index.crates.io-")):
        raise ValueError("expected one private crates.io archive cache")
    cache_relative = Path("registry/cache") / cache_roots[0].name
    before = private_home.stat(follow_symlinks=False)
    root_fd = os.open(private_home, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        if file_identity(os.fstat(root_fd)) != file_identity(before):
            raise ValueError("private Cargo home changed before comparison")
        reports = []
        total = 0
        for filename in sorted(selected):
            if filename not in locked:
                raise ValueError(f"selected archive absent from lockfile: {filename}")
            name, version, checksum = locked[filename]
            expected = selected[filename]
            if (not isinstance(expected, dict) or set(expected) != {"sha256", "bytes"}
                    or expected["sha256"] != checksum
                    or not isinstance(expected["bytes"], int)
                    or isinstance(expected["bytes"], bool)
                    or not 0 < expected["bytes"] <= MAX_ARCHIVE):
                raise ValueError(f"invalid selected archive identity: {filename}")
            total += expected["bytes"]
            if total > MAX_TOTAL:
                raise ValueError("selected archives exceed total size bound")
            with ordinary_file_beneath(root_fd, cache_relative / filename, MAX_ARCHIVE) as local:
                info = os.fstat(local.fileno())
                local_identity = stream_identity(local, expected["bytes"])
                if file_identity(os.fstat(local.fileno())) != file_identity(info):
                    raise ValueError(f"private archive changed during read: {filename}")
            if local_identity != expected:
                raise ValueError(f"private archive differs from candidate: {filename}")
            url = f"https://static.crates.io/crates/{name}/{filename}"
            with fetch(url) as remote:
                official_identity = stream_identity(remote, expected["bytes"])
            if official_identity != expected:
                raise ValueError(f"official archive differs from lockfile: {filename}")
            reports.append({"url": url, "sha256": checksum, "bytes": expected["bytes"]})
        if file_identity(private_home.stat(follow_symlinks=False)) != file_identity(before):
            raise ValueError("private Cargo home changed during comparison")
    finally:
        os.close(root_fd)
    if root_inventory(private_home) != candidate["privateRoot"]:
        raise ValueError("private Cargo home changed during comparison")
    return {"format": 2, "target": candidate["target"],
            "lockSha256": candidate["lockSha256"],
            "privateRootSha256": candidate["privateRoot"]["sha256"],
            "selectionSha256": selection_sha256(candidate),
            "archives": reports, "buildReady": False}


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
