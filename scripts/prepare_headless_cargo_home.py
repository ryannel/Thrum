"""Prepare a private, inspectable Cargo home for the first native build profile.

This prepares compiler inputs; its manifest is not a trusted BuildRevision.
The caller supplies an already materialized accepted source tree and a trusted
private parent for the new destination.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import threading
import tomllib

REGISTRY_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"
CHUNK = 1024 * 1024
MAX_METADATA_BYTES = 32 * 1024 * 1024
MAX_DIAGNOSTIC_BYTES = 1024 * 1024
SEMVER = re.compile(
    r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)"
    r"(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
)


def sha256_file(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(CHUNK), b""):
            digest.update(chunk)
    return digest.hexdigest()


def index_key(name):
    if not re.fullmatch(r"[a-z0-9_-]+", name):
        raise ValueError(f"unsupported registry crate name: {name!r}")
    if len(name) == 1:
        return Path("1") / name
    if len(name) == 2:
        return Path("2") / name
    if len(name) == 3:
        return Path("3") / name[0] / name
    return Path(name[:2]) / name[2:4] / name


def copy_checked(source, destination, expected=None):
    if not source.is_file() or source.is_symlink():
        raise ValueError(f"missing or aliased dependency input: {source}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    digest = hashlib.sha256()
    length = 0
    with source.open("rb") as reader, destination.open("xb") as writer:
        for chunk in iter(lambda: reader.read(CHUNK), b""):
            writer.write(chunk)
            digest.update(chunk)
            length += len(chunk)
    actual = digest.hexdigest()
    if expected is not None and actual != expected:
        raise ValueError(f"dependency checksum mismatch: {source}")
    return {"sha256": actual, "bytes": length}


def cargo_metadata(cargo, rustc, source, cargo_home, target, scratch):
    environment = {
        "HOME": str(cargo_home.parent),
        "CARGO_HOME": str(cargo_home),
        "RUSTC": str(rustc),
        "RUSTC_WRAPPER": "",
        "RUSTC_WORKSPACE_WRAPPER": "",
        "RUSTFLAGS": "",
        "CARGO_NET_OFFLINE": "true",
        "CARGO_TARGET_DIR": str(scratch / "target"),
        "TMPDIR": str(scratch / "tmp"),
        "PATH": os.pathsep.join((str(rustc.parent), os.defpath)),
        "LC_ALL": "C",
    }
    command = [
        str(cargo), "metadata", "--locked", "--offline", "--format-version", "1",
        "--filter-platform", target, "--manifest-path", str(source / "Cargo.toml"),
    ]
    if sys.platform == "darwin":
        profile = scratch / "metadata.sb"
        profile.write_text(
            '(version 1)\n(deny default)\n(deny network*)\n(allow process*)\n'
            '(allow mach-lookup)\n(allow sysctl-read)\n(allow file-read-metadata)\n'
            '(allow file-read* (literal "/") (subpath "/System") '
            '(subpath "/usr") (subpath "/Library") (subpath "/Applications") '
            '(subpath "/opt") (subpath "/private/etc") (subpath "/var/select") '
            '(subpath "/private/var/select") (subpath "/dev") '
            '(subpath (param "SOURCE")) (subpath (param "CARGO_HOME")) '
            '(subpath (param "TOOLCHAIN")))\n'
            '(allow file-write* (subpath (param "CARGO_HOME")) '
            '(literal "/dev/null"))\n'
        )
        command = [
            "/usr/bin/sandbox-exec",
            "-D", f"SOURCE={source.resolve()}",
            "-D", f"CARGO_HOME={cargo_home.resolve()}",
            "-D", f"TOOLCHAIN={cargo.parent.parent.resolve()}",
            "-f", str(profile),
            *command,
        ]
    status, output, error = collect_bounded(command, source, environment)
    if status:
        raise ValueError(
            "locked offline Cargo metadata failed: "
            + error[-4096:].decode("utf-8", errors="replace")
        )
    return json.loads(output)


def collect_bounded(
    command, cwd, environment,
    stdout_limit=MAX_METADATA_BYTES, stderr_limit=MAX_DIAGNOSTIC_BYTES,
):
    """Drain both pipes while limiting retained bytes and killing overproducing Cargo."""
    process = subprocess.Popen(
        command, cwd=cwd, env=environment, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        start_new_session=os.name == "posix",
    )
    output = bytearray()
    error = bytearray()
    exceeded = threading.Event()

    def terminate_group():
        try:
            if os.name == "posix":
                os.killpg(process.pid, signal.SIGKILL)
            else:
                process.kill()
        except ProcessLookupError:
            pass

    def drain(pipe, retained, limit):
        for chunk in iter(lambda: os.read(pipe.fileno(), 64 * 1024), b""):
            if len(retained) + len(chunk) > limit:
                exceeded.set()
                terminate_group()
                break
            retained.extend(chunk)

    readers = [
        threading.Thread(target=drain, args=(process.stdout, output, stdout_limit), daemon=True),
        threading.Thread(target=drain, args=(process.stderr, error, stderr_limit), daemon=True),
    ]
    for reader in readers:
        reader.start()
    timed_out = False
    try:
        status = process.wait(timeout=120)
    except subprocess.TimeoutExpired:
        timed_out = True
        terminate_group()
        process.wait()
        status = process.returncode
    for reader in readers:
        reader.join(timeout=0.2)
    held_pipes = any(reader.is_alive() for reader in readers)
    if held_pipes:
        terminate_group()
        for reader in readers:
            reader.join(timeout=1)
    if any(reader.is_alive() for reader in readers):
        raise ValueError("Cargo metadata descendants retained output pipes")
    try:
        process.stdout.close()
        process.stderr.close()
    finally:
        if timed_out:
            raise ValueError("locked offline Cargo metadata timed out")
    if held_pipes:
        raise ValueError("Cargo metadata descendants retained output pipes")
    if exceeded.is_set():
        raise ValueError("Cargo metadata exceeds the first-profile output limit")
    return status, bytes(output), bytes(error)


def root_inventory(root):
    digest = hashlib.sha256(b"thrum.cargo-home.v1\0")
    count = total = 0
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            raise ValueError(f"private cache contains a symlink: {path}")
        if not path.is_file():
            continue
        relative = path.relative_to(root).as_posix().encode("utf-8")
        length = path.stat().st_size
        digest.update(len(relative).to_bytes(4, "little"))
        digest.update(relative)
        digest.update(length.to_bytes(8, "little"))
        digest.update(bytes.fromhex(sha256_file(path)))
        count += 1
        total += length
    return {"sha256": digest.hexdigest(), "files": count, "bytes": total}


def prepare(source, shared_home, destination, cargo, rustc, target):
    for label, path in (
        ("source", source), ("shared Cargo home", shared_home),
        ("destination", destination), ("Cargo", cargo), ("Rustc", rustc),
    ):
        if not path.is_absolute():
            raise ValueError(f"{label} must be absolute: {path}")
    if not source.is_dir() or not shared_home.is_dir() or not cargo.is_file() or not rustc.is_file():
        raise ValueError("source, shared Cargo home and selected compiler binaries must exist")
    if source.is_symlink() or cargo.parent != rustc.parent:
        raise ValueError("source must be an ordinary tree and Cargo/Rustc one toolchain")
    if destination.exists() or destination.is_symlink():
        raise ValueError("private Cargo home destination must be absent")
    if destination.parent.resolve().is_relative_to(source.resolve()):
        raise ValueError("private Cargo home must be outside the materialized source tree")
    # Cargo searches these locations even when CARGO_HOME is explicit. The
    # first profile refuses implicit replacement sources and executable hooks.
    for ancestor in (source, *source.parents):
        for name in ("config", "config.toml"):
            config = ancestor / ".cargo" / name
            if config.exists() or config.is_symlink():
                raise ValueError(f"ambient Cargo configuration is unsupported: {config}")
    lock_bytes = (source / "Cargo.lock").read_bytes()
    lock = tomllib.loads(lock_bytes.decode("utf-8"))
    if lock.get("version") != 4 or not isinstance(lock.get("package"), list):
        raise ValueError("unsupported Cargo lockfile format")
    registry = {}
    for package in lock["package"]:
        package_source = package.get("source")
        if package_source is None:
            continue
        if package_source != REGISTRY_SOURCE:
            raise ValueError(f"unsupported dependency source: {package_source}")
        name, version, checksum = package["name"], package["version"], package["checksum"]
        if len(name) > 128 or len(version) > 128 or not SEMVER.fullmatch(version):
            raise ValueError(f"unsupported registry package spelling: {name} {version}")
        index_key(name)
        if not re.fullmatch(r"[0-9a-f]{64}", checksum):
            raise ValueError(f"invalid lockfile checksum for {name}")
        key = (name, version)
        if key in registry:
            raise ValueError(f"duplicate registry package: {name} {version}")
        registry[key] = checksum
    index_roots = list((shared_home / "registry/index").glob("index.crates.io-*"))
    if len(index_roots) != 1 or index_roots[0].is_symlink():
        raise ValueError("expected one ordinary crates.io sparse-index cache")
    index_root = index_roots[0]
    cache_root = shared_home / "registry/cache" / index_root.name
    if not cache_root.is_dir() or cache_root.is_symlink():
        raise ValueError("crates.io archive cache is missing or aliased")

    manifest_path = destination.parent / f"{destination.name}.manifest.json"
    if manifest_path.exists() or manifest_path.is_symlink():
        raise ValueError(f"cache manifest already exists: {manifest_path}")
    destination.mkdir()
    manifest_created = False
    try:
        scratch = destination / "_prepare"
        (scratch / "tmp").mkdir(parents=True)
        private_index = destination / "registry/index" / index_root.name
        private_cache = destination / "registry/cache" / index_root.name
        copied_index = {}
        copy_checked(index_root / "config.json", private_index / "config.json")
        for name in sorted({name for name, _ in registry}):
            key = Path(".cache") / index_key(name)
            copied_index[name] = copy_checked(index_root / key, private_index / key)
        verified_archives = {}
        for name, version in sorted(registry):
            filename = f"{name}-{version}.crate"
            verified_archives[filename] = copy_checked(
                cache_root / filename, private_cache / filename, registry[(name, version)]
            )
        # Resolve only after all lockfile archives and index records are copied.
        # Cargo never executes with the shared home or its configuration.
        metadata = cargo_metadata(cargo, rustc, source, destination, target, scratch)
        for filename, identity in verified_archives.items():
            if sha256_file(private_cache / filename) != identity["sha256"]:
                raise ValueError(f"private archive changed during Cargo resolution: {filename}")
        for name, identity in copied_index.items():
            key = Path(".cache") / index_key(name)
            if sha256_file(private_index / key) != identity["sha256"]:
                raise ValueError(f"private index changed during Cargo resolution: {name}")
        selected = set()
        for package in metadata["packages"]:
            package_source = package["source"]
            manifest = Path(package["manifest_path"]).resolve()
            if package_source is None:
                if not manifest.is_relative_to(source.resolve()):
                    raise ValueError(f"local package escapes admitted source: {manifest}")
            elif package_source == REGISTRY_SOURCE:
                key = (package["name"], package["version"])
                if key not in registry:
                    raise ValueError(f"resolved package absent from lockfile: {key}")
                if not manifest.is_relative_to((destination / "registry/src").resolve()):
                    raise ValueError(f"registry package escapes private cache: {manifest}")
                selected.add(key)
            else:
                raise ValueError(f"unsupported resolved source: {package_source}")
        copied_archives = {}
        for name, version in sorted(registry):
            filename = f"{name}-{version}.crate"
            if (name, version) in selected:
                copied_archives[filename] = verified_archives[filename]
            else:
                (private_cache / filename).unlink()
        # The compacted cache must still resolve without deleted archives.
        private_metadata = cargo_metadata(cargo, rustc, source, destination, target, scratch)
        if private_metadata["resolve"] != metadata["resolve"] or {
            package["id"] for package in private_metadata["packages"]
        } != {package["id"] for package in metadata["packages"]}:
            raise ValueError("private Cargo resolution changed after cache pruning")
        shutil.rmtree(scratch)
        # Cargo's usage database contains host/path-specific cache bookkeeping;
        # the locked offline compile can recreate or omit it. It is not input.
        (destination / ".global-cache").unlink(missing_ok=True)
        inventory = root_inventory(destination)
        manifest = {
            "format": 1,
            "target": target,
            "lockSha256": hashlib.sha256(lock_bytes).hexdigest(),
            "registryIndexRecords": copied_index,
            "registryArchives": copied_archives,
            "verifiedLockArchives": len(verified_archives),
            "privateRoot": inventory,
        }
        with manifest_path.open("x", encoding="utf-8") as output:
            manifest_created = True
            output.write(json.dumps(manifest, sort_keys=True, separators=(",", ":")) + "\n")
        return manifest_path, inventory
    except BaseException:
        if manifest_created:
            manifest_path.unlink(missing_ok=True)
        shutil.rmtree(destination)
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("source", "shared-home", "destination", "cargo", "rustc", "target"):
        parser.add_argument(f"--{name}", required=True)
    args = parser.parse_args()
    manifest, inventory = prepare(
        Path(args.source), Path(args.shared_home), Path(args.destination),
        Path(args.cargo), Path(args.rustc), args.target,
    )
    print(json.dumps({"manifest": str(manifest), "privateRoot": inventory}, sort_keys=True))


if __name__ == "__main__":
    main()
