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
import stat
import subprocess
import sys
import threading
import tomllib

REGISTRY_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"
REUSE_DOMAIN = b"thrum.headless-cargo-reuse.v1\0"
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
    try:
        for reader in readers:
            reader.start()
        try:
            status = process.wait(timeout=120)
        except subprocess.TimeoutExpired:
            terminate_group()
            process.wait()
            raise ValueError("locked offline Cargo metadata timed out")
        for reader in readers:
            reader.join(timeout=0.2)
        held_pipes = any(reader.is_alive() for reader in readers)
        if held_pipes:
            terminate_group()
            for reader in readers:
                reader.join(timeout=1)
        if any(reader.is_alive() for reader in readers):
            raise ValueError("Cargo metadata descendants retained output pipes")
        if held_pipes:
            raise ValueError("Cargo metadata descendants retained output pipes")
        if exceeded.is_set():
            raise ValueError("Cargo metadata exceeds the first-profile output limit")
        return status, bytes(output), bytes(error)
    except BaseException:
        # Ctrl-C also lands here. The child starts a new session, so terminal
        # interruption alone does not stop it before the caller removes scratch.
        terminate_group()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
        for reader in readers:
            if reader.ident is not None:
                reader.join(timeout=1)
        raise
    finally:
        process.stdout.close()
        process.stderr.close()


def root_inventory(root):
    if os.name != "posix":
        raise ValueError("private Cargo inventory requires POSIX no-follow reads")
    if not root.is_absolute() or root.is_symlink() or not root.is_dir():
        raise ValueError("private Cargo home must be an ordinary absolute directory")
    root = root.resolve(strict=True)

    def identity(info):
        return (
            info.st_dev, info.st_ino, info.st_mode, info.st_size,
            info.st_mtime_ns, info.st_ctime_ns,
        )

    nofollow = os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK
    directory_flags = nofollow | os.O_DIRECTORY
    records = []

    def visit(directory_fd, parts):
        with os.scandir(directory_fd) as entries:
            names = sorted(entry.name for entry in entries)
        for name in names:
            relative_name = "/".join((*parts, name))
            before = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
            if stat.S_ISDIR(before.st_mode):
                child_fd = os.open(name, directory_flags, dir_fd=directory_fd)
                try:
                    if identity(before) != identity(os.fstat(child_fd)):
                        raise ValueError(f"private cache directory changed: {relative_name}")
                    visit(child_fd, (*parts, name))
                finally:
                    os.close(child_fd)
            elif stat.S_ISREG(before.st_mode):
                fd = os.open(name, nofollow, dir_fd=directory_fd)
                with os.fdopen(fd, "rb") as stream:
                    opened = os.fstat(stream.fileno())
                    if identity(before) != identity(opened):
                        raise ValueError(f"private cache file changed before hashing: {relative_name}")
                    content = hashlib.sha256()
                    length = 0
                    for chunk in iter(lambda: stream.read(CHUNK), b""):
                        content.update(chunk)
                        length += len(chunk)
                    if identity(opened) != identity(os.fstat(stream.fileno())):
                        raise ValueError(f"private cache file changed while hashing: {relative_name}")
                if length != before.st_size:
                    raise ValueError(f"private cache file length changed: {relative_name}")
                records.append(((*parts, name), os.fsencode(relative_name), length, content.digest()))
            else:
                raise ValueError(f"private cache contains a non-file entry: {relative_name}")
            if identity(before) != identity(os.stat(
                name, dir_fd=directory_fd, follow_symlinks=False,
            )):
                raise ValueError(f"private cache entry changed during inventory: {relative_name}")

    root_fd = os.open(root, directory_flags)
    try:
        before_root = identity(os.fstat(root_fd))
        if before_root != identity(os.stat(root, follow_symlinks=False)):
            raise ValueError("private cache root changed before inventory")
        visit(root_fd, ())
        if before_root != identity(os.fstat(root_fd)) or before_root != identity(
            os.stat(root, follow_symlinks=False)
        ):
            raise ValueError("private cache root changed during inventory")
    finally:
        os.close(root_fd)

    digest = hashlib.sha256(b"thrum.cargo-home.v1\0")
    total = 0
    for _, relative, length, content_digest in sorted(records):
        digest.update(len(relative).to_bytes(4, "little"))
        digest.update(relative)
        digest.update(length.to_bytes(8, "little"))
        digest.update(content_digest)
        total += length
    return {"sha256": digest.hexdigest(), "files": len(records), "bytes": total}


def possible_headless_packages(metadata, source):
    """Walk normal/build edges; workspace feature union can overstate build units."""
    packages = {package["id"]: package for package in metadata["packages"]}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    root_manifest = (source / "examples/i1-mobile-game/Cargo.toml").resolve()
    roots = [
        package["id"] for package in packages.values()
        if package["name"] == "i1-mobile-game"
        and package["source"] is None
        and Path(package["manifest_path"]).resolve() == root_manifest
    ]
    if len(roots) != 1:
        raise ValueError("expected one admitted headless package")
    selected = set()
    pending = roots
    while pending:
        package_id = pending.pop()
        if package_id in selected:
            continue
        selected.add(package_id)
        if package_id not in packages or package_id not in nodes:
            raise ValueError(f"selected package missing from metadata: {package_id}")
        for dependency in nodes[package_id]["deps"]:
            if any(
                kind["kind"] in (None, "build")
                for kind in dependency["dep_kinds"]
            ):
                pending.append(dependency["pkg"])
    records = []
    for package_id in selected:
        package = packages[package_id]
        if any(
            kind in ("custom-build", "proc-macro")
            for target in package["targets"] for kind in target["kind"]
        ):
            raise ValueError(f"first headless profile refuses executable dependency hooks: {package_id}")
        manifest = Path(package["manifest_path"]).resolve()
        if package["source"] is None:
            relative = manifest.parent.relative_to(source.resolve()).as_posix()
            origin = f"path:{relative}"
        elif package["source"] == REGISTRY_SOURCE:
            origin = package["source"]
        else:
            raise ValueError(f"unsupported selected dependency source: {package_id}")
        records.append({
            "origin": origin,
            "name": package["name"],
            "version": package["version"],
            "features": sorted(nodes[package_id]["features"]),
        })
    records.sort(key=lambda record: (record["origin"], record["name"], record["version"]))
    return records


def cargo_reuse_key(source, target, tool_executables):
    """Bind the pruned cache to dependency-selection inputs, not code edits."""
    if not source.is_absolute() or source.is_symlink() or not source.is_dir():
        raise ValueError("admitted source must be an ordinary absolute directory")
    source = source.resolve(strict=True)
    for ancestor in (source, *source.parents):
        for name in ("config", "config.toml"):
            config = ancestor / ".cargo" / name
            if config.exists() or config.is_symlink():
                raise ValueError(f"ambient Cargo configuration is unsupported: {config}")
    for path in source.rglob("build.rs"):
        raise ValueError(f"first headless profile refuses implicit build scripts: {path}")
    manifests = []
    for path in sorted(source.rglob("Cargo.toml")):
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"aliased or missing Cargo manifest: {path}")
        manifests.append({
            "path": path.relative_to(source).as_posix(),
            "sha256": sha256_file(path),
        })
    if not any(manifest["path"] == "Cargo.toml" for manifest in manifests):
        raise ValueError("root Cargo manifest is missing")
    toolchain = source / "rust-toolchain.toml"
    if toolchain.is_symlink() or not toolchain.is_file():
        raise ValueError("pinned Rust toolchain declaration is missing or aliased")
    declaration = source / "build/headless.json"
    if declaration.is_symlink():
        raise ValueError("headless build declaration is aliased")
    lockfile = source / "Cargo.lock"
    if lockfile.is_symlink() or not lockfile.is_file():
        raise ValueError("Cargo lockfile is missing or aliased")
    inputs = {
        "cargoManifests": manifests,
        "lockSha256": sha256_file(lockfile),
        "toolchainDeclarationSha256": sha256_file(toolchain),
        "buildDeclarationSha256": sha256_file(declaration) if declaration.is_file() else None,
        "target": target,
        "recipe": {
            "package": "i1-mobile-game", "bin": "headless",
            "profile": "dev", "features": [], "incremental": False,
            "locked": True, "offline": True, "rustflags": "",
        },
        "toolExecutables": tool_executables,
    }
    encoded = json.dumps(inputs, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return {"sha256": hashlib.sha256(REUSE_DOMAIN + encoded).hexdigest(), "inputs": inputs}


def prepare(source, shared_home, destination, cargo, rustc, target):
    for label, path in (
        ("source", source), ("shared Cargo home", shared_home),
        ("destination", destination), ("Cargo", cargo), ("Rustc", rustc),
    ):
        if not path.is_absolute():
            raise ValueError(f"{label} must be absolute: {path}")
    if not source.is_dir() or not shared_home.is_dir() or not cargo.is_file() or not rustc.is_file():
        raise ValueError("source, shared Cargo home and selected compiler binaries must exist")
    if source.is_symlink() or cargo.is_symlink() or rustc.is_symlink() or cargo.parent != rustc.parent:
        raise ValueError("source must be an ordinary tree and Cargo/Rustc one toolchain")
    if destination.exists() or destination.is_symlink():
        raise ValueError("private Cargo home destination must be absent")
    if destination.parent.resolve().is_relative_to(source.resolve()):
        raise ValueError("private Cargo home must be outside the materialized source tree")
    # Cargo selection inputs are checked again after resolution and before reuse.
    # Do the initial check before creating the private home as well.
    cargo_reuse_key(source, target, {})
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
        tool_executables = {
            name: {"path": str(path.resolve()), "sha256": sha256_file(path),
                   "bytes": path.stat().st_size}
            for name, path in (("cargo", cargo), ("rustc", rustc))
        }
        reuse_key = cargo_reuse_key(source, target, tool_executables)
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
        possible_packages = possible_headless_packages(metadata, source)
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
        for name, path in (("cargo", cargo), ("rustc", rustc)):
            if sha256_file(path) != tool_executables[name]["sha256"]:
                raise ValueError(f"selected {name} executable changed during cache preparation")
        if cargo_reuse_key(source, target, tool_executables) != reuse_key:
            raise ValueError("Cargo dependency selection inputs changed during preparation")
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
            "possibleBuildPackages": possible_packages,
            "toolExecutables": tool_executables,
            "reuseKey": reuse_key,
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
    parser.add_argument("--source", required=True)
    parser.add_argument("--verify-reuse", type=Path)
    parser.add_argument("--private-home", type=Path)
    for name in ("shared-home", "destination", "cargo", "rustc", "target"):
        parser.add_argument(f"--{name}")
    args = parser.parse_args()
    if args.verify_reuse is not None:
        if not Path(args.source).is_absolute():
            raise ValueError("admitted source must be absolute for reuse verification")
        for name in ("cargo", "rustc", "target", "private_home"):
            if getattr(args, name) is None:
                parser.error(f"--{name.replace('_', '-')} is required for reuse verification")
        candidate = json.loads(args.verify_reuse.read_text(encoding="utf-8"))
        if candidate["format"] != 1:
            raise ValueError("unsupported private Cargo candidate format")
        if args.target != candidate["target"]:
            raise ValueError("private Cargo cache target differs from selected target")
        selected_tools = {}
        for name in ("cargo", "rustc"):
            path = Path(getattr(args, name))
            if not path.is_absolute() or path.is_symlink() or not path.is_file():
                raise ValueError(f"selected {name} is not an ordinary absolute executable")
            selected_tools[name] = {
                "path": str(path.resolve()), "sha256": sha256_file(path),
                "bytes": path.stat().st_size,
            }
        if selected_tools != candidate["toolExecutables"]:
            raise ValueError("private Cargo cache tool executables differ from selected tools")
        actual = cargo_reuse_key(Path(args.source), args.target, selected_tools)
        if actual != candidate["reuseKey"]:
            raise ValueError("private Cargo cache cannot be reused for this source")
        if root_inventory(args.private_home) != candidate["privateRoot"]:
            raise ValueError("private Cargo cache bytes changed since preparation")
        print(json.dumps({"reuseKey": actual["sha256"]}, sort_keys=True))
        return
    for name in ("shared_home", "destination", "cargo", "rustc", "target"):
        if getattr(args, name) is None:
            parser.error(f"--{name.replace('_', '-')} is required for preparation")
    manifest, inventory = prepare(
        Path(args.source), Path(args.shared_home), Path(args.destination),
        Path(args.cargo), Path(args.rustc), args.target,
    )
    print(json.dumps({"manifest": str(manifest), "privateRoot": inventory}, sort_keys=True))


if __name__ == "__main__":
    main()
