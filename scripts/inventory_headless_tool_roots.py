"""Inventory selected native tool roots as candidate release inputs.

This captures observed bytes; no release signature or build readiness follows.
Run once per installed tool/SDK selection, outside the per-edit build path.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import posixpath
import stat

CHUNK = 1024 * 1024
DOMAIN = b"thrum.tool-root.v1\0"


def inventory_root(root):
    if os.name != "posix":
        raise ValueError("this candidate inventory requires POSIX descriptor-relative reads")
    if not root.is_absolute() or root.is_symlink() or not root.is_dir():
        raise ValueError(f"tool root must be an ordinary absolute directory: {root}")
    root = root.resolve(strict=True)
    digest = hashlib.sha256(DOMAIN)
    files = links = directories = total_bytes = 0
    recorded_links = []
    nofollow = os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK
    directory_flags = nofollow | os.O_DIRECTORY

    def identity(info):
        return (
            info.st_dev, info.st_ino, info.st_mode, info.st_size,
            info.st_mtime_ns, info.st_ctime_ns,
        )

    def visit(directory_fd, parts):
        nonlocal files, links, directories, total_bytes
        with os.scandir(directory_fd) as entries:
            names = sorted(entry.name for entry in entries)
        for name in names:
            path_parts = (*parts, name)
            relative_name = "/".join(path_parts)
            relative = os.fsencode(relative_name)
            if len(relative) > 0xFFFF_FFFF:
                raise ValueError(f"tool path is too long: {relative_name}")
            before = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
            digest.update(len(relative).to_bytes(4, "little"))
            digest.update(relative)
            if stat.S_ISDIR(before.st_mode):
                child_fd = os.open(name, directory_flags, dir_fd=directory_fd)
                try:
                    if identity(before) != identity(os.fstat(child_fd)):
                        raise ValueError(f"tool directory changed: {relative_name}")
                    directories += 1
                    digest.update(b"d")
                    visit(child_fd, path_parts)
                finally:
                    os.close(child_fd)
            elif stat.S_ISLNK(before.st_mode):
                target = os.readlink(name, dir_fd=directory_fd)
                if posixpath.isabs(target):
                    try:
                        contained = Path(target).resolve(strict=True).is_relative_to(root)
                    except (OSError, RuntimeError) as error:
                        raise ValueError(f"unresolved tool symlink: {relative_name}") from error
                else:
                    normalized = posixpath.normpath(posixpath.join(*parts, target))
                    contained = normalized != ".." and not normalized.startswith("../")
                if not contained:
                    raise ValueError(f"tool symlink escapes selected root: {relative_name}")
                link = os.fsencode(target)
                if len(link) > 0xFFFF_FFFF:
                    raise ValueError(f"tool symlink is too long: {relative_name}")
                links += 1
                digest.update(b"l")
                digest.update(len(link).to_bytes(4, "little"))
                digest.update(link)
                recorded_links.append(relative_name)
            elif stat.S_ISREG(before.st_mode):
                file_fd = os.open(name, nofollow, dir_fd=directory_fd)
                with os.fdopen(file_fd, "rb") as stream:
                    if identity(before) != identity(os.fstat(stream.fileno())):
                        raise ValueError(f"tool file changed before hashing: {relative_name}")
                    content = hashlib.sha256()
                    length = 0
                    for chunk in iter(lambda: stream.read(CHUNK), b""):
                        content.update(chunk)
                        length += len(chunk)
                    if identity(before) != identity(os.fstat(stream.fileno())):
                        raise ValueError(f"tool file changed while hashing: {relative_name}")
                if length != before.st_size:
                    raise ValueError(f"tool file length changed: {relative_name}")
                files += 1
                total_bytes += length
                digest.update(b"f")
                digest.update(length.to_bytes(8, "little"))
                digest.update(content.digest())
            else:
                raise ValueError(f"unsupported tool entry: {relative_name}")
            if identity(before) != identity(os.stat(name, dir_fd=directory_fd, follow_symlinks=False)):
                raise ValueError(f"tool entry changed during inventory: {relative_name}")

    root_fd = os.open(root, directory_flags)
    try:
        root_identity = identity(os.fstat(root_fd))
        if root_identity != identity(os.stat(root, follow_symlinks=False)):
            raise ValueError("tool root changed before inventory")
        visit(root_fd, ())
        for relative_name in recorded_links:
            try:
                target = (root / relative_name).resolve(strict=True)
            except (OSError, RuntimeError) as error:
                raise ValueError(f"unresolved tool symlink: {relative_name}") from error
            if not target.is_relative_to(root):
                raise ValueError(f"tool symlink escapes selected root: {relative_name}")
        if root_identity != identity(os.fstat(root_fd)) or root_identity != identity(
            os.stat(root, follow_symlinks=False)
        ):
            raise ValueError("tool root changed during inventory")
    finally:
        os.close(root_fd)
    return {
        "sha256": digest.hexdigest(),
        "files": files,
        "symlinks": links,
        "directories": directories,
        "logicalBytes": total_bytes,
    }


def inventory_roots(roots):
    if not roots or len(roots) != len({path.resolve() for path in roots.values()}):
        raise ValueError("select distinct named tool roots")
    return {name: inventory_root(path) for name, path in sorted(roots.items())}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("rust-toolchain", "linker-toolchain", "sdk"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if not args.output.is_absolute() or not args.output.parent.is_dir():
        parser.error("output needs an absolute path under an existing directory")
    roots = {
        "rustToolchain": args.rust_toolchain,
        "linkerToolchain": args.linker_toolchain,
        "sdk": args.sdk,
    }
    if any(args.output.resolve().is_relative_to(root.resolve()) for root in roots.values()):
        parser.error("output must be outside selected tool roots")
    inventory = {"format": 1, "roots": inventory_roots(roots)}
    created = False
    try:
        with args.output.open("x", encoding="utf-8") as stream:
            created = True
            stream.write(json.dumps(inventory, sort_keys=True, separators=(",", ":")) + "\n")
            stream.flush()
            os.fsync(stream.fileno())
    except BaseException:
        if created:
            args.output.unlink(missing_ok=True)
        raise
    print(json.dumps(inventory, sort_keys=True))


if __name__ == "__main__":
    main()
