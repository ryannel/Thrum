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
from release_cargo_selection import selection_sha256
from verify_crates_io_archive_origin import locked_archives
from verify_rust_component_archive import safe_relative


PROFILE = "headless-macos-arm64-dev-v1"
TARGET = "aarch64-apple-darwin"
ROOT_NAMES = ("rustToolchain", "linkerToolchain", "sdk")
CARGO_PROOFS = ("cargoArchives", "cargoIndex", "cargoExtraction")
RUST_PROOFS = {"rustCargo": "cargo", "rustc": "rustc", "rustStd": "rust-std"}
PROOF_NAMES = (*CARGO_PROOFS, *RUST_PROOFS)
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
    exact_fields(record, ("format", "sources", "proofs"), "origin evidence")
    if type(record["format"]) is not int or record["format"] != 3:
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
    exact_fields(record["proofs"], PROOF_NAMES, "origin evidence proofs")
    files = set()
    for name, proof in record["proofs"].items():
        exact_fields(proof, ("file", "sha256"), f"origin proof {name}")
        filename = proof["file"]
        if (not isinstance(filename, str) or not EVIDENCE_FILE.fullmatch(filename)
                or filename in (".", "..") or filename in files):
            raise ValueError(f"origin proof {name} must use a distinct adjacent file")
        files.add(filename)
        sha256_hex(proof["sha256"], f"origin proof {name} SHA-256")


def verify_origin_proofs(record, evidence_path, manifest, candidate, lock_bytes):
    """Bind reviewed reports to selected inputs; return Rust file witnesses."""
    root = manifest["dependencyRoot"]["sha256"]
    selection = selection_sha256(candidate)
    reports = {}
    for name in PROOF_NAMES:
        descriptor = record["proofs"][name]
        if descriptor["file"] == evidence_path.name:
            raise ValueError("origin proof cannot refer to the evidence record itself")
        path = evidence_path.parent / descriptor["file"]
        report, digest = canonical_json_file(path, 512 * 1024)
        if digest != descriptor["sha256"]:
            raise ValueError(f"origin proof bytes differ: {name}")
        if (not isinstance(report, dict) or type(report.get("format")) is not int
                or report["format"] != (2 if name in CARGO_PROOFS else 3)
                or report.get("target") != TARGET
                or report.get("buildReady") is not False):
            raise ValueError(f"origin proof selection differs: {name}")
        if name in CARGO_PROOFS and (
            report.get("privateRootSha256") != root
            or report.get("selectionSha256") != selection
        ):
            raise ValueError(f"origin proof selection differs: {name}")
        reports[name] = report

    archives = reports["cargoArchives"]
    exact_fields(archives, (
        "format", "target", "lockSha256", "privateRootSha256", "selectionSha256",
        "archives", "buildReady",
    ), "cargo archive origin proof")
    if archives["lockSha256"] != manifest["lockSha256"]:
        raise ValueError("cargo archive proof lockfile differs")
    selected_archives = candidate.get("registryArchives")
    if not isinstance(selected_archives, dict) or not selected_archives:
        raise ValueError("private cache lacks selected archive identities")
    locked = locked_archives(lock_bytes)
    expected_archives = []
    for filename in sorted(selected_archives):
        if filename not in locked:
            raise ValueError(f"selected archive absent from lockfile: {filename}")
        name, _, checksum = locked[filename]
        identity = selected_archives[filename]
        if (not isinstance(identity, dict) or set(identity) != {"sha256", "bytes"}
                or identity["sha256"] != checksum or type(identity["bytes"]) is not int
                or identity["bytes"] <= 0):
            raise ValueError(f"invalid private archive identity: {filename}")
        expected_archives.append({
            "url": f"https://static.crates.io/crates/{name}/{filename}",
            "sha256": checksum, "bytes": identity["bytes"],
        })
    if archives["archives"] != expected_archives:
        raise ValueError("cargo archive proof differs from selected cache")

    index = reports["cargoIndex"]
    exact_fields(index, (
        "format", "target", "privateRootSha256", "selectionSha256", "indexNames",
        "cachedVersionsMatched", "buildReady",
    ), "cargo index origin proof")
    index_records = candidate.get("registryIndexRecords")
    if not isinstance(index_records, dict) or not index_records:
        raise ValueError("private cache lacks selected index identities")
    count(index["indexNames"], "cargo index proof names")
    count(index["cachedVersionsMatched"], "cargo index proof versions")
    if (index["indexNames"] != len(index_records)
            or index["cachedVersionsMatched"] < index["indexNames"]):
        raise ValueError("cargo index proof differs from selected cache")

    extraction = reports["cargoExtraction"]
    exact_fields(extraction, (
        "format", "target", "privateRootSha256", "selectionSha256", "packages",
        "matchedFiles", "matchedBytes", "buildReady",
    ), "cargo extraction proof")
    for field in ("packages", "matchedFiles", "matchedBytes"):
        count(extraction[field], f"cargo extraction proof {field}")
    if (extraction["packages"] != len(selected_archives)
            or extraction["matchedFiles"] == 0 or extraction["matchedBytes"] == 0):
        raise ValueError("cargo extraction proof differs from selected cache")

    rust_files = {}
    channel_hash = None
    for name, component in RUST_PROOFS.items():
        report = reports[name]
        exact_fields(report, (
            "format", "component", "target", "channelSha256", "archiveSha256",
            "matchedFiles", "matchedBytes", "installedFiles", "installedMetadata",
            "buildReady",
        ), f"Rust component proof {name}")
        if report["component"] != component:
            raise ValueError(f"Rust component proof selection differs: {name}")
        sha256_hex(report["channelSha256"], f"Rust component {name} channel")
        sha256_hex(report["archiveSha256"], f"Rust component {name} archive")
        if channel_hash is None:
            channel_hash = report["channelSha256"]
        elif report["channelSha256"] != channel_hash:
            raise ValueError("Rust component proofs have different channel manifests")
        files = report["installedFiles"]
        count(report["matchedFiles"], f"Rust component {name} files")
        count(report["matchedBytes"], f"Rust component {name} bytes")
        if not isinstance(files, list) or not 0 < len(files) <= 10000:
            raise ValueError(f"Rust component {name} has no bounded file witness")
        previous = ""
        total = 0
        payload_paths = set()
        for item in files:
            exact_fields(item, ("path", "sha256", "bytes"), f"Rust component {name} file")
            path = item["path"]
            if not isinstance(path, str) or path <= previous or safe_relative(path).as_posix() != path:
                raise ValueError(f"Rust component {name} file paths are not sorted and unique")
            previous = path
            sha256_hex(item["sha256"], f"Rust component {name} file hash")
            count(item["bytes"], f"Rust component {name} file bytes")
            if path in rust_files:
                raise ValueError("Rust component proofs overlap installed files")
            payload_paths.add(path)
            rust_files[path] = {"sha256": item["sha256"], "bytes": item["bytes"]}
            total += item["bytes"]
        if report["matchedFiles"] != len(files) or report["matchedBytes"] != total:
            raise ValueError(f"Rust component {name} totals differ from file witnesses")
        metadata = report["installedMetadata"]
        required = {
            "lib/rustlib/multirust-channel-manifest.toml",
            "lib/rustlib/components",
            f"lib/rustlib/manifest-{component}-{TARGET}",
        }
        if not isinstance(metadata, list) or len(metadata) != len(required):
            raise ValueError(f"Rust component {name} installer metadata witness differs")
        previous_metadata = ""
        for item in metadata:
            exact_fields(item, ("path", "sha256", "bytes"),
                         f"Rust component {name} installer metadata")
            path = item["path"]
            if (not isinstance(path, str) or path not in required or path in payload_paths
                    or path <= previous_metadata):
                raise ValueError(f"Rust component {name} installer metadata path differs")
            previous_metadata = path
            required.remove(path)
            sha256_hex(item["sha256"], f"Rust component {name} installer metadata hash")
            count(item["bytes"], f"Rust component {name} installer metadata bytes")
            identity = {"sha256": item["sha256"], "bytes": item["bytes"]}
            if path in rust_files and rust_files[path] != identity:
                raise ValueError("Rust component proofs disagree on installed metadata")
            rust_files[path] = identity
        if required:
            raise ValueError(f"Rust component {name} installer metadata witness missing")
    return rust_files


def selected_file_in_root(path, root, label):
    if not path.is_absolute() or path.is_symlink() or not path.is_file():
        raise ValueError(f"selected {label} is not an ordinary absolute file")
    if not path.resolve(strict=True).is_relative_to(root.resolve(strict=True)):
        raise ValueError(f"selected {label} escapes its release root")


def load_release_selection(manifest_path, source, cache_manifest_path, target):
    """Validate canonical release records and their cache/source relationships."""
    if os.name != "posix":
        raise ValueError("first native release input verifier requires POSIX")
    if not source.is_absolute() or not source.is_dir() or source.is_symlink():
        raise ValueError("admitted source must be an ordinary absolute directory")
    manifest, manifest_hash = canonical_json_file(manifest_path, 64 * 1024)
    validate_manifest(manifest)
    if manifest_path.resolve().is_relative_to(source.resolve()):
        raise ValueError("reviewed release manifest must be outside source")
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
    candidate, cache_manifest_hash = canonical_json_file(cache_manifest_path, 512 * 1024)
    if candidate["lockSha256"] != manifest["lockSha256"]:
        raise ValueError("admitted lockfile differs from reviewed release selection")
    lock_bytes = bounded_regular_bytes(source / "Cargo.lock", 4 * 1024 * 1024)
    if hashlib.sha256(lock_bytes).hexdigest() != manifest["lockSha256"]:
        raise ValueError("admitted lockfile bytes differ from reviewed release manifest")
    rust_files = verify_origin_proofs(record, evidence, manifest, candidate, lock_bytes)
    if candidate["privateRoot"] != manifest["dependencyRoot"]:
        raise ValueError("private dependency root differs from reviewed release manifest")
    return manifest, manifest_hash, candidate, cache_manifest_hash, rust_files


def verify_release_inputs(manifest_path, source, cache_manifest_path, private_home,
                          rust_toolchain, linker_toolchain, sdk, cargo, rustc,
                          linker, target):
    """Cold exact-byte match; reviewed origin and compiler closure remain external."""
    if not private_home.is_absolute() or not private_home.is_dir() or private_home.is_symlink():
        raise ValueError("private Cargo home must be an ordinary absolute directory")
    if manifest_path.resolve().is_relative_to(private_home.resolve()):
        raise ValueError("reviewed release manifest must be outside cache")
    manifest, manifest_hash, candidate, _, rust_files = load_release_selection(
        manifest_path, source, cache_manifest_path, target,
    )
    selected_file_in_root(cargo, rust_toolchain, "Cargo")
    selected_file_in_root(rustc, rust_toolchain, "Rustc")
    selected_file_in_root(linker, linker_toolchain, "linker")
    actual_key = verify_reuse(source, candidate, private_home, cargo, rustc, target)
    if actual_key["inputs"]["lockSha256"] != manifest["lockSha256"]:
        raise ValueError("admitted lockfile bytes differ from reviewed release manifest")
    if actual_key["sha256"] != manifest["selectionKey"]:
        raise ValueError("admitted build selection differs from reviewed release manifest")
    roots, observed = inventory_roots({
        "rustToolchain": rust_toolchain,
        "linkerToolchain": linker_toolchain,
        "sdk": sdk,
    }, {"rustToolchain": rust_files})
    if roots != manifest["toolRoots"]:
        raise ValueError("native tool roots differ from reviewed release manifest")
    if observed["rustToolchain"] != rust_files:
        raise ValueError("Rust component receipts differ from selected tool root")
    return {
        "format": 1, "profile": PROFILE, "manifestSha256": manifest_hash,
        "selectionKey": actual_key["sha256"], "matchedInputRoots": True,
        "matchedOriginProofs": True,
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
