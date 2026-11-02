"""Match first-profile native build inputs to a caller-selected release manifest.

The caller must select a manifest from a reviewed engine release. This tool
checks exact local bytes and selection; it cannot establish the manifest's
origin or make an executable a ready BuildRevision.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
from urllib.parse import urlsplit

from inventory_headless_tool_roots import inventory_roots
from prepare_headless_cargo_home import verify_reuse


PROFILE = "headless-macos-arm64-dev-v1"
TARGET = "aarch64-apple-darwin"
ROOT_NAMES = ("rustToolchain", "linkerToolchain", "sdk")
ORIGIN_HOSTS = {
    "rustToolchain": {"static.rust-lang.org"},
    "linkerToolchain": {"developer.apple.com", "download.developer.apple.com", "apps.apple.com"},
    "sdk": {"developer.apple.com", "download.developer.apple.com", "apps.apple.com"},
    "cargoRegistry": {"index.crates.io", "static.crates.io"},
}
HEX = re.compile(r"[0-9a-f]{64}\Z")
EVIDENCE_FILE = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}\Z")


def exact_fields(value, fields, label):
    if not isinstance(value, dict) or set(value) != set(fields):
        raise ValueError(f"{label} has unsupported or missing fields")


def sha256_hex(value, label):
    if not isinstance(value, str) or not HEX.fullmatch(value):
        raise ValueError(f"{label} must be a lowercase SHA-256 digest")


def count(value, label):
    if type(value) is not int or value < 0:
        raise ValueError(f"{label} must be a nonnegative integer")


def strict_pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON field: {key}")
        result[key] = value
    return result


def file_identity(info):
    return (
        info.st_dev, info.st_ino, info.st_mode, info.st_size,
        info.st_mtime_ns, info.st_ctime_ns,
    )


def bounded_regular_bytes(path, maximum):
    if not path.is_absolute() or path.is_symlink():
        raise ValueError(f"expected an ordinary absolute manifest file: {path}")
    parent = path.parent.resolve(strict=True)
    flags = os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK
    directory_fd = os.open(parent, flags | os.O_DIRECTORY)
    try:
        before = os.stat(path.name, dir_fd=directory_fd, follow_symlinks=False)
        if not stat.S_ISREG(before.st_mode):
            raise ValueError(f"expected a regular manifest file: {path}")
        file_fd = os.open(path.name, flags, dir_fd=directory_fd)
        with os.fdopen(file_fd, "rb") as stream:
            opened = os.fstat(stream.fileno())
            if file_identity(before) != file_identity(opened):
                raise ValueError(f"manifest file changed before read: {path}")
            raw = stream.read(maximum + 1)
            if file_identity(opened) != file_identity(os.fstat(stream.fileno())):
                raise ValueError(f"manifest file changed during read: {path}")
        if file_identity(before) != file_identity(os.stat(
            path.name, dir_fd=directory_fd, follow_symlinks=False,
        )):
            raise ValueError(f"manifest file changed after read: {path}")
    finally:
        os.close(directory_fd)
    if len(raw) > maximum:
        raise ValueError(f"manifest exceeds {maximum} bytes: {path}")
    return raw


def canonical_json_file(path, maximum):
    raw = bounded_regular_bytes(path, maximum)
    return canonical_json_bytes(raw, path)


def canonical_json_bytes(raw, label):
    value = json.loads(
        raw, object_pairs_hook=strict_pairs,
        parse_constant=lambda name: (_ for _ in ()).throw(ValueError(f"invalid JSON number: {name}")),
    )
    canonical = (json.dumps(value, sort_keys=True, separators=(",", ":"),
                            ensure_ascii=True, allow_nan=False) + "\n").encode("utf-8")
    if raw != canonical:
        raise ValueError(f"manifest is not canonical JSON: {label}")
    return value, hashlib.sha256(raw).hexdigest()


def validate_manifest(manifest):
    exact_fields(manifest, (
        "format", "profile", "target", "selectionKey", "lockSha256",
        "dependencyRoot", "toolRoots", "originEvidence",
    ), "release input manifest")
    if type(manifest["format"]) is not int or manifest["format"] != 1:
        raise ValueError("unsupported release input manifest format")
    if manifest["profile"] != PROFILE or manifest["target"] != TARGET:
        raise ValueError("unsupported first native release profile")
    sha256_hex(manifest["selectionKey"], "selectionKey")
    sha256_hex(manifest["lockSha256"], "lockSha256")
    exact_fields(manifest["dependencyRoot"], ("sha256", "files", "bytes"), "dependencyRoot")
    sha256_hex(manifest["dependencyRoot"]["sha256"], "dependencyRoot.sha256")
    count(manifest["dependencyRoot"]["files"], "dependencyRoot.files")
    count(manifest["dependencyRoot"]["bytes"], "dependencyRoot.bytes")
    exact_fields(manifest["toolRoots"], ROOT_NAMES, "toolRoots")
    for name in ROOT_NAMES:
        root = manifest["toolRoots"][name]
        exact_fields(root, (
            "sha256", "files", "symlinks", "directories", "logicalBytes",
        ), f"toolRoots.{name}")
        sha256_hex(root["sha256"], f"toolRoots.{name}.sha256")
        for field in ("files", "symlinks", "directories", "logicalBytes"):
            count(root[field], f"toolRoots.{name}.{field}")
    exact_fields(manifest["originEvidence"], ("file", "sha256"), "originEvidence")
    if not isinstance(manifest["originEvidence"]["file"], str) or not EVIDENCE_FILE.fullmatch(
        manifest["originEvidence"]["file"]
    ) or manifest["originEvidence"]["file"] in (".", ".."):
        raise ValueError("origin evidence must be one ordinary adjacent file")
    sha256_hex(manifest["originEvidence"]["sha256"], "originEvidence.sha256")


def validate_origin_evidence(record):
    exact_fields(record, ("format", "sources"), "origin evidence")
    if type(record["format"]) is not int or record["format"] != 1:
        raise ValueError("unsupported origin evidence format")
    exact_fields(record["sources"], ORIGIN_HOSTS, "origin evidence sources")
    for name, source in record["sources"].items():
        exact_fields(source, ("url", "version"), f"origin evidence {name}")
        if not isinstance(source["url"], str):
            raise ValueError(f"origin evidence {name} URL must be HTTPS")
        parsed = urlsplit(source["url"])
        if parsed.scheme != "https" or parsed.hostname not in ORIGIN_HOSTS[name] or (
            parsed.username is not None or parsed.password is not None or parsed.port is not None
        ):
            raise ValueError(f"origin evidence {name} URL is not an official HTTPS origin")
        if not isinstance(source["version"], str) or not 1 <= len(source["version"]) <= 128:
            raise ValueError(f"origin evidence {name} version is missing or too long")


def selected_file_in_root(path, root, label):
    if not path.is_absolute() or path.is_symlink() or not path.is_file():
        raise ValueError(f"selected {label} is not an ordinary absolute file")
    if not path.resolve(strict=True).is_relative_to(root.resolve(strict=True)):
        raise ValueError(f"selected {label} escapes its release root")


def verify_release_inputs(manifest_path, source, cache_manifest_path, private_home,
                          rust_toolchain, linker_toolchain, sdk, cargo, rustc,
                          linker, target):
    """Cold exact-byte match; reviewed origin and compiler closure remain external."""
    if os.name != "posix":
        raise ValueError("first native release input verifier requires POSIX")
    if not source.is_absolute() or not source.is_dir() or source.is_symlink():
        raise ValueError("admitted source must be an ordinary absolute directory")
    if not private_home.is_absolute() or not private_home.is_dir() or private_home.is_symlink():
        raise ValueError("private Cargo home must be an ordinary absolute directory")
    manifest, manifest_hash = canonical_json_file(manifest_path, 64 * 1024)
    validate_manifest(manifest)
    if manifest_path.resolve().is_relative_to(source.resolve()) or manifest_path.resolve().is_relative_to(
        private_home.resolve()
    ):
        raise ValueError("reviewed release manifest must be outside source and cache")
    if target != manifest["target"]:
        raise ValueError("caller target differs from reviewed release manifest")
    evidence = manifest_path.parent / manifest["originEvidence"]["file"]
    evidence_bytes = bounded_regular_bytes(evidence, 64 * 1024)
    if hashlib.sha256(evidence_bytes).hexdigest() != manifest[
        "originEvidence"
    ]["sha256"]:
        raise ValueError("reviewed origin evidence bytes differ")
    record, _ = canonical_json_bytes(evidence_bytes, evidence)
    validate_origin_evidence(record)
    candidate, _ = canonical_json_file(cache_manifest_path, 512 * 1024)
    if candidate["lockSha256"] != manifest["lockSha256"]:
        raise ValueError("admitted lockfile differs from reviewed release selection")
    selected_file_in_root(cargo, rust_toolchain, "Cargo")
    selected_file_in_root(rustc, rust_toolchain, "Rustc")
    selected_file_in_root(linker, linker_toolchain, "linker")
    actual_key = verify_reuse(source, candidate, private_home, cargo, rustc, target)
    if actual_key["inputs"]["lockSha256"] != manifest["lockSha256"]:
        raise ValueError("admitted lockfile bytes differ from reviewed release manifest")
    if actual_key["sha256"] != manifest["selectionKey"]:
        raise ValueError("admitted build selection differs from reviewed release manifest")
    if candidate["privateRoot"] != manifest["dependencyRoot"]:
        raise ValueError("private dependency root differs from reviewed release manifest")
    roots = inventory_roots({
        "rustToolchain": rust_toolchain,
        "linkerToolchain": linker_toolchain,
        "sdk": sdk,
    })
    if roots != manifest["toolRoots"]:
        raise ValueError("native tool roots differ from reviewed release manifest")
    return {
        "format": 1, "profile": PROFILE, "manifestSha256": manifest_hash,
        "selectionKey": actual_key["sha256"], "matchedInputRoots": True,
        "buildReady": False,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in (
        "manifest", "source", "cache-manifest", "private-home", "rust-toolchain",
        "linker-toolchain", "sdk", "cargo", "rustc", "linker", "target",
    ):
        parser.add_argument(f"--{name}", required=True)
    args = parser.parse_args()
    report = verify_release_inputs(
        Path(args.manifest), Path(args.source), Path(args.cache_manifest),
        Path(args.private_home), Path(args.rust_toolchain),
        Path(args.linker_toolchain), Path(args.sdk), Path(args.cargo),
        Path(args.rustc), Path(args.linker), args.target,
    )
    print(json.dumps(report, sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()
