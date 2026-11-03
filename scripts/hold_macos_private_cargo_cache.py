"""Unready held macOS candidate for one pruned private Cargo cache.

The caller supplies independently selected identities. This helper does not
authenticate registry origin or confer build readiness. A failed preparation
leaves its digest name unusable rather than repairing a published selection.
"""

import hashlib
import argparse
import json
import os
from pathlib import Path
import stat
import sys

from hold_macos_tool_root_clone import (
    _check_entry, _checked_parent, _names, _seal_fd, open_sealed_clone,
    prepare_sealed_clone,
)
from inventory_headless_tool_roots import inventory_root
from prepare_headless_cargo_home import cargo_reuse_key, root_inventory, sha256_file

_MAX_MANIFEST_BYTES = 4 * 1024 * 1024
_MAX_RECORD_BYTES = 2048
_NOFOLLOW_FILE = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)


def _digest(value):
    return (isinstance(value, str) and len(value) == 64
            and all(char in "0123456789abcdef" for char in value))


def _manifest_bytes(path):
    path = Path(path)
    if not path.is_absolute():
        raise ValueError("private-cache manifest path must be absolute")
    fd = os.open(path, _NOFOLLOW_FILE)
    try:
        before = os.fstat(fd)
        named = os.stat(path, follow_symlinks=False)
        if (not stat.S_ISREG(before.st_mode) or before.st_uid != os.geteuid()
                or before.st_nlink != 1 or before.st_size > _MAX_MANIFEST_BYTES
                or (before.st_dev, before.st_ino) != (named.st_dev, named.st_ino)):
            raise ValueError("private-cache manifest has an untrusted identity")
        with os.fdopen(fd, "rb", closefd=False) as stream:
            raw = stream.read(_MAX_MANIFEST_BYTES + 1)
        after = os.fstat(fd)
        if (len(raw) != before.st_size or len(raw) > _MAX_MANIFEST_BYTES
                or (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns,
                    before.st_ctime_ns) != (after.st_dev, after.st_ino, after.st_size,
                                            after.st_mtime_ns, after.st_ctime_ns)):
            raise ValueError("private-cache manifest changed during selection")
        candidate = json.loads(raw)
        canonical = (json.dumps(candidate, sort_keys=True, separators=(",", ":")) + "\n").encode()
        if raw != canonical or candidate.get("format") != 2:
            raise ValueError("private-cache manifest is not the selected canonical format")
        legacy = candidate.get("privateRoot")
        if (not isinstance(legacy, dict) or set(legacy) != {"sha256", "files", "bytes"}
                or not _digest(legacy["sha256"])
                or any(type(legacy[key]) is not int or legacy[key] < 0
                       for key in ("files", "bytes"))):
            raise ValueError("private-cache manifest has an invalid root identity")
        return raw, legacy
    finally:
        os.close(fd)


def _paths(parent, tree):
    root_name, _, _ = _names(tree)
    digest = tree["sha256"]
    return (Path(parent) / root_name,
            Path(parent) / f".cargo-manifest-{digest}.json",
            Path(parent) / f".cargo-selection-{digest}.json")


def _record_bytes(tree, legacy, manifest_sha):
    if not _digest(manifest_sha):
        raise ValueError("invalid private-cache manifest digest")
    return (json.dumps({
        "buildReady": False,
        "format": 1,
        "legacyCache": legacy,
        "manifestSha256": manifest_sha,
        "tree": tree,
    }, sort_keys=True, separators=(",", ":")) + "\n").encode("ascii")


def _publish_sealed_file(path, raw):
    pending = path.with_name(path.name + ".pending")
    fd = os.open(pending, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    try:
        with os.fdopen(fd, "wb", closefd=False) as stream:
            stream.write(raw)
            stream.flush()
        os.fsync(fd)
    finally:
        os.close(fd)
    os.rename(pending, path)
    fd = os.open(path, _NOFOLLOW_FILE)
    try:
        _seal_fd(fd, False)
    finally:
        os.close(fd)


def prepare_sealed_cache(source, manifest_path, owner_parent):
    """Cold clone and seal; return the three selected identities and path."""
    if sys.platform != "darwin":
        raise ValueError("held private Cargo cache requires macOS")
    owner = _checked_parent(Path(owner_parent))
    raw, legacy = _manifest_bytes(manifest_path)
    if root_inventory(Path(source)) != legacy:
        raise ValueError("private-cache bytes differ from the source manifest")
    tree = inventory_root(Path(source))
    root, copied_manifest, record = _paths(owner, tree)
    if (copied_manifest.exists() or copied_manifest.is_symlink()
            or record.exists() or record.is_symlink()):
        raise ValueError("private-cache selection already exists or is incomplete")
    prepared, _ = prepare_sealed_clone(Path(source), owner, tree)
    if prepared != root or root_inventory(root) != legacy or _manifest_bytes(manifest_path)[0] != raw:
        raise ValueError("private-cache source changed during sealing")
    manifest_sha = hashlib.sha256(raw).hexdigest()
    _publish_sealed_file(copied_manifest, raw)
    _publish_sealed_file(record, _record_bytes(tree, legacy, manifest_sha))
    parent_fd = os.open(owner, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        os.fsync(parent_fd)
    finally:
        os.close(parent_fd)
    return root, tree, legacy, manifest_sha


def _open_sealed_file(path, maximum, expected):
    fd = os.open(path, _NOFOLLOW_FILE)
    try:
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode):
            raise ValueError("private-cache selection file is not ordinary")
        _check_entry(info, False)
        if info.st_size > maximum:
            raise ValueError("private-cache selection file is too large")
        named = os.stat(path, follow_symlinks=False)
        if (info.st_dev, info.st_ino) != (named.st_dev, named.st_ino):
            raise ValueError("private-cache selection file changed identity")
        with os.fdopen(fd, "rb", closefd=False) as stream:
            raw = stream.read(maximum + 1)
        if raw != expected or os.fstat(fd).st_size != len(raw):
            raise ValueError("private-cache selection file differs from expected bytes")
        return fd
    except BaseException:
        os.close(fd)
        raise


class HeldCargoCache:
    """Cold-verified cache with a cheap owner-held identity check."""

    def __init__(self, root_lease, files, manifest_bytes, legacy):
        self.root = root_lease.root
        self.manifest_bytes = manifest_bytes
        self.legacy = legacy.copy()
        self._root_lease = root_lease
        self._files = files

    def assert_live(self):
        self._root_lease.assert_live()
        for path, fd in self._files:
            held = os.fstat(fd)
            named = os.stat(path, follow_symlinks=False)
            if ((held.st_dev, held.st_ino) != (named.st_dev, named.st_ino)
                    or not held.st_flags & stat.UF_IMMUTABLE):
                raise ValueError("private-cache lease lost its selected file")

    def close(self):
        files, self._files = self._files, ()
        first_error = None
        try:
            for _, fd in files:
                try:
                    os.close(fd)
                except OSError as error:
                    if first_error is None:
                        first_error = error
        finally:
            self._root_lease.close()
        if first_error is not None:
            raise first_error

    def __enter__(self):
        self.assert_live()
        return self

    def __exit__(self, _exc_type, _exc_value, _traceback):
        self.close()


def open_sealed_cache(owner_parent, tree, legacy, manifest_sha):
    """Cold verify both digest domains and exact manifest under one root lease."""
    if sys.platform != "darwin":
        raise ValueError("held private Cargo cache requires macOS")
    owner = _checked_parent(Path(owner_parent))
    _record_bytes(tree, legacy, manifest_sha)
    _root, copied_manifest, record = _paths(owner, tree)
    root_lease = open_sealed_clone(owner, tree)
    files = []
    try:
        record_fd = _open_sealed_file(
            record, _MAX_RECORD_BYTES, _record_bytes(tree, legacy, manifest_sha),
        )
        files.append((record, record_fd))
        manifest_fd = os.open(copied_manifest, _NOFOLLOW_FILE)
        try:
            info = os.fstat(manifest_fd)
            if not stat.S_ISREG(info.st_mode):
                raise ValueError("private-cache manifest copy is not ordinary")
            _check_entry(info, False)
            if info.st_size > _MAX_MANIFEST_BYTES:
                raise ValueError("private-cache manifest copy is too large")
            named = os.stat(copied_manifest, follow_symlinks=False)
            if (info.st_dev, info.st_ino) != (named.st_dev, named.st_ino):
                raise ValueError("private-cache manifest copy changed identity")
            with os.fdopen(manifest_fd, "rb", closefd=False) as stream:
                raw = stream.read(_MAX_MANIFEST_BYTES + 1)
            if (len(raw) != info.st_size or hashlib.sha256(raw).hexdigest() != manifest_sha
                    or json.loads(raw)["privateRoot"] != legacy):
                raise ValueError("private-cache manifest copy differs from selected identity")
            files.append((copied_manifest, manifest_fd))
        except BaseException:
            os.close(manifest_fd)
            raise
        if root_inventory(root_lease.root) != legacy:
            raise ValueError("sealed private cache differs from its legacy identity")
        selection = HeldCargoCache(root_lease, files, raw, legacy)
        selection.assert_live()
        return selection
    except BaseException:
        for _, fd in files:
            os.close(fd)
        root_lease.close()
        raise


class HeldNativeCacheSelection:
    """Cold-checked native roots; warm reuse hashes only source keys."""

    def __init__(self, cache, rust_root, linker_root, sdk_root, rust_witnesses=None):
        cache.assert_live()
        rust_root.assert_live()
        linker_root.assert_live()
        sdk_root.assert_live()
        candidate = json.loads(cache.manifest_bytes)
        if (not isinstance(candidate, dict) or candidate.get("format") != 2
                or candidate.get("privateRoot") != cache.legacy
                or not isinstance(candidate.get("target"), str)
                or not isinstance(candidate.get("reuseKey"), dict)
                or not _digest(candidate["reuseKey"].get("sha256"))
                or not isinstance(candidate.get("toolExecutables"), dict)
                or set(candidate["toolExecutables"]) != {"cargo", "rustc"}):
            raise ValueError("held cache manifest has an incompatible dependency selection")
        selected_tools = candidate["toolExecutables"]
        for name in ("cargo", "rustc"):
            path = rust_root.root / "bin" / name
            identity = selected_tools[name]
            witnessed = None if rust_witnesses is None else rust_witnesses.get(f"bin/{name}")
            if (not isinstance(identity, dict)
                    or set(identity) != {"path", "sha256", "bytes"}
                    or identity["path"] != str(path)
                    or not _digest(identity["sha256"])
                    or type(identity["bytes"]) is not int or identity["bytes"] < 0
                    or path.is_symlink() or not path.is_file()
                    or path.stat().st_size != identity["bytes"]
                    or (witnessed != {"sha256": identity["sha256"], "bytes": identity["bytes"]}
                        if rust_witnesses is not None
                        else sha256_file(path) != identity["sha256"])):
                raise ValueError(f"held {name} differs from cache tool selection")
        cache.assert_live()
        rust_root.assert_live()
        self.cache = cache
        self.rust_root = rust_root
        self.linker_root = linker_root
        self.sdk_root = sdk_root
        self.candidate = candidate
        self.selected_tools = selected_tools

    def verify_reuse(self, source, target):
        self.cache.assert_live()
        self.rust_root.assert_live()
        self.linker_root.assert_live()
        self.sdk_root.assert_live()
        if target != self.candidate["target"]:
            raise ValueError("held private cache target differs from selected target")
        actual = cargo_reuse_key(Path(source), target, self.selected_tools)
        if actual != self.candidate["reuseKey"]:
            raise ValueError("held private cache cannot be reused for this source")
        self.cache.assert_live()
        self.rust_root.assert_live()
        self.linker_root.assert_live()
        self.sdk_root.assert_live()
        return actual


class HeldNativeReleaseSelection:
    """An unready reviewed-record candidate bound to four held native roots."""

    def __init__(self, release, cache, rust_root, linker_root, sdk_root):
        release.assert_live()
        expected = release.manifest["toolRoots"]
        actual = {
            "rustToolchain": rust_root.expected,
            "linkerToolchain": linker_root.expected,
            "sdk": sdk_root.expected,
        }
        if expected != actual or rust_root.witnesses != release.rust_witnesses:
            raise ValueError("held release tool roots differ from reviewed selection")
        if release.manifest["dependencyRoot"] != cache.legacy:
            raise ValueError("held release cache root differs from reviewed selection")
        if release.cache_manifest_sha256 != hashlib.sha256(cache.manifest_bytes).hexdigest():
            raise ValueError("held cache manifest differs from reviewed release candidate")
        self.native = HeldNativeCacheSelection(
            cache, rust_root, linker_root, sdk_root, release.rust_witnesses,
        )
        if self.native.candidate["reuseKey"]["sha256"] != release.manifest["selectionKey"]:
            raise ValueError("held release dependency key differs from reviewed selection")
        self.release = release

    def verify_reuse(self, source, target):
        self.release.assert_live()
        actual = self.native.verify_reuse(source, target)
        if actual["sha256"] != self.release.manifest["selectionKey"]:
            raise ValueError("held release dependency key changed")
        self.release.assert_live()
        return actual

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--prepare", action="store_true")
    mode.add_argument("--hold", action="store_true")
    parser.add_argument("--source", type=Path)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--owner", type=Path, required=True)
    parser.add_argument("--tree-json")
    parser.add_argument("--legacy-json")
    parser.add_argument("--rust-tree-json")
    parser.add_argument("--linker-tree-json")
    parser.add_argument("--sdk-tree-json")
    parser.add_argument("--manifest-sha")
    args = parser.parse_args()
    if args.prepare:
        if (args.source is None or args.manifest is None
                or any(value is not None for value in (
                    args.tree_json, args.legacy_json, args.rust_tree_json,
                    args.linker_tree_json, args.sdk_tree_json,
                    args.manifest_sha,
                ))):
            parser.error("preparation needs source and manifest only")
        root, tree, legacy, manifest_sha = prepare_sealed_cache(
            args.source, args.manifest, args.owner,
        )
        selected_manifest = json.loads(_manifest_bytes(_paths(args.owner, tree)[1])[0])
        reuse_sha = selected_manifest.get("reuseKey", {}).get("sha256")
        if not _digest(reuse_sha):
            raise ValueError("sealed private cache lacks a reuse-key digest")
        print(json.dumps({"root": str(root), "tree": tree, "legacy": legacy,
                          "manifestSha256": manifest_sha, "reuseKeySha256": reuse_sha},
                         sort_keys=True), flush=True)
    else:
        if (args.source is not None or args.manifest is not None
                or any(value is None for value in (
                    args.tree_json, args.legacy_json, args.rust_tree_json,
                    args.linker_tree_json, args.sdk_tree_json,
                    args.manifest_sha,
                ))):
            parser.error("holding needs tree, legacy and manifest digest only")
        with (open_sealed_cache(
                args.owner, json.loads(args.tree_json), json.loads(args.legacy_json),
                args.manifest_sha,
              ) as cache_lease,
              open_sealed_clone(args.owner, json.loads(args.rust_tree_json)) as rust_lease,
              open_sealed_clone(args.owner, json.loads(args.linker_tree_json)) as linker_lease,
              open_sealed_clone(args.owner, json.loads(args.sdk_tree_json)) as sdk_lease):
            selected = HeldNativeCacheSelection(
                cache_lease, rust_lease, linker_lease, sdk_lease,
            )
            print("READY", flush=True)
            while raw := sys.stdin.buffer.readline(4097):
                if len(raw) > 4096 or not raw.endswith(b"\n"):
                    raise ValueError("held-cache request exceeds selected size")
                request = json.loads(raw)
                if (not isinstance(request, dict) or set(request) != {"id", "source", "target"}
                        or type(request["id"]) is not int or request["id"] < 0
                        or request["id"] > 0xFFFF_FFFF_FFFF_FFFF
                        or not isinstance(request["source"], str)
                        or not isinstance(request["target"], str)):
                    raise ValueError("invalid held-cache reuse request")
                try:
                    actual = selected.verify_reuse(request["source"], request["target"])
                    response = {"id": request["id"], "reuseKey": actual["sha256"]}
                except (OSError, ValueError) as error:
                    response = {"id": request["id"], "error": str(error)[:512]}
                print(json.dumps(response, sort_keys=True, separators=(",", ":")), flush=True)
            selected.cache.assert_live()
            selected.rust_root.assert_live()
            selected.linker_root.assert_live()
            selected.sdk_root.assert_live()


if __name__ == "__main__":
    main()
