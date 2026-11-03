"""Prepare an unready, exact-inventory APFS clone of one selected tool root.

The caller supplies the independently selected expected inventory. This helper
does not authenticate that inventory, prevent another process from writing to
the clone later, or publish a ready compiler environment.
"""

import ctypes
import os
from pathlib import Path
import shutil
import stat
import sys

from inventory_headless_tool_roots import inventory_root

_FIELDS = {"sha256", "files", "symlinks", "directories", "logicalBytes"}


def _identity(info):
    return (info.st_dev, info.st_ino, info.st_mode, info.st_size,
            info.st_mtime_ns, info.st_ctime_ns)


def _directory_identity(info):
    return (info.st_dev, info.st_ino, info.st_mode)


def _clone_function():
    if sys.platform != "darwin":
        raise ValueError("APFS tool-root cloning requires macOS")
    clone = ctypes.CDLL(None, use_errno=True).fclonefileat
    clone.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_char_p, ctypes.c_int]
    clone.restype = ctypes.c_int
    return clone


def prepare_clone(source, destination, expected):
    """Clone and inventory a root on one volume; return only on an exact match.

    `destination` must be absent under a private, ordinary parent controlled by
    the caller. The complete clone is deleted on a failed preparation unless
    its directory identity changed, in which case the caller must quarantine
    that parent. Successful preparation does not make the root immutable.
    """
    source = Path(source)
    destination = Path(destination)
    if (set(expected) != _FIELDS or not isinstance(expected["sha256"], str)
            or len(expected["sha256"]) != 64
            or any(char not in "0123456789abcdef" for char in expected["sha256"])
            or any(type(expected[name]) is not int or expected[name] < 0
                   for name in _FIELDS - {"sha256"})):
        raise ValueError("invalid expected tool-root inventory")
    clone_file = _clone_function()
    open_directory = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_NONBLOCK
    open_file = os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK
    if (not source.is_absolute() or not destination.is_absolute()
            or source.is_symlink() or not source.is_dir()
            or destination.exists() or destination.is_symlink()):
        raise ValueError("source must be an ordinary absolute directory; destination absent")
    parent = destination.parent
    if not parent.is_dir() or parent.is_symlink():
        raise ValueError("destination parent must be an ordinary directory")
    parent = parent.resolve(strict=True)
    destination = parent / destination.name
    source = source.resolve(strict=True)
    if destination.is_relative_to(source) or source.is_relative_to(destination):
        raise ValueError("tool source and clone must be disjoint")
    if os.stat(source).st_dev != os.stat(parent).st_dev:
        raise ValueError("APFS clone destination must be on the source volume")

    counts = {"files": 0, "symlinks": 0, "directories": 0, "logicalBytes": 0}

    def increment(name, amount=1):
        counts[name] += amount
        if counts[name] > expected[name]:
            raise ValueError(f"tool root exceeds expected {name}")

    def walk(source_fd, destination_fd, depth):
        if depth > 128:
            raise ValueError("tool root exceeds clone depth")
        with os.scandir(source_fd) as entries:
            for entry in entries:
                name = entry.name
                before = os.stat(name, dir_fd=source_fd, follow_symlinks=False)
                before_identity = _identity(before)
                if stat.S_ISDIR(before.st_mode):
                    increment("directories")
                    child_fd = os.open(name, open_directory, dir_fd=source_fd)
                    try:
                        if _identity(os.fstat(child_fd)) != _identity(before):
                            raise ValueError("tool directory changed during clone")
                        os.mkdir(name, mode=0o700, dir_fd=destination_fd)
                        clone_fd = os.open(name, open_directory, dir_fd=destination_fd)
                        try:
                            walk(child_fd, clone_fd, depth + 1)
                        finally:
                            os.close(clone_fd)
                        if _identity(os.fstat(child_fd)) != _identity(before):
                            raise ValueError("tool directory changed during clone")
                    finally:
                        os.close(child_fd)
                elif stat.S_ISLNK(before.st_mode):
                    increment("symlinks")
                    target = os.readlink(name, dir_fd=source_fd)
                    if os.path.isabs(target):
                        raise ValueError("absolute tool-root link needs an explicit mapping")
                    os.symlink(target, name, dir_fd=destination_fd)
                elif stat.S_ISREG(before.st_mode):
                    increment("files")
                    increment("logicalBytes", before.st_size)
                    file_fd = os.open(name, open_file, dir_fd=source_fd)
                    try:
                        if _identity(os.fstat(file_fd)) != _identity(before):
                            raise ValueError("tool file changed before clone")
                        if clone_file(file_fd, destination_fd, os.fsencode(name), 0) != 0:
                            error = ctypes.get_errno()
                            raise OSError(error, os.strerror(error), name)
                        if _identity(os.fstat(file_fd)) != _identity(before):
                            raise ValueError("tool file changed during clone")
                    finally:
                        os.close(file_fd)
                else:
                    raise ValueError("unsupported tool-root entry")
                if _identity(os.stat(name, dir_fd=source_fd, follow_symlinks=False)) != before_identity:
                    raise ValueError("tool entry changed during clone")

    source_fd = os.open(source, open_directory)
    created = False
    destination_identity = None
    try:
        os.mkdir(destination, mode=0o700)
        created = True
        destination_identity = _directory_identity(os.stat(destination, follow_symlinks=False))
        destination_fd = os.open(destination, open_directory)
        try:
            walk(source_fd, destination_fd, 0)
        finally:
            os.close(destination_fd)
        observed = inventory_root(destination)
        if observed != expected or any(observed[name] != value for name, value in counts.items()):
            raise ValueError("cloned tool-root inventory differs from selected bytes")
        return observed
    except BaseException:
        if created:
            current = os.stat(destination, follow_symlinks=False)
            if _directory_identity(current) != destination_identity:
                raise RuntimeError("clone destination changed; quarantine its parent")
            shutil.rmtree(destination)
        raise
    finally:
        os.close(source_fd)
