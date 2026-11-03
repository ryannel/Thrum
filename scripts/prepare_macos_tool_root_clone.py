"""Prepare an unready, exact-inventory APFS clone of one selected tool root.

The caller supplies the independently selected expected inventory. This helper
does not authenticate that inventory, prevent another process from writing to
the clone later, or publish a ready compiler environment.
"""

import ctypes
import errno
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


def _extended_acl_status(fd):
    if sys.platform != "darwin":
        raise ValueError("APFS tool-root cloning requires macOS")
    libc = ctypes.CDLL(None, use_errno=True)
    get_acl = libc.acl_get_fd_np
    get_acl.argtypes = [ctypes.c_int, ctypes.c_int]
    get_acl.restype = ctypes.c_void_p
    free_acl = libc.acl_free
    free_acl.argtypes = [ctypes.c_void_p]
    free_acl.restype = ctypes.c_int
    get_entry = libc.acl_get_entry
    get_entry.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.POINTER(ctypes.c_void_p)]
    get_entry.restype = ctypes.c_int
    get_tag = libc.acl_get_tag_type
    get_tag.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_int)]
    get_tag.restype = ctypes.c_int
    ctypes.set_errno(0)
    acl = get_acl(fd, 0x100)  # ACL_TYPE_EXTENDED from the macOS SDK.
    if acl:
        try:
            entry = ctypes.c_void_p()
            ctypes.set_errno(0)
            result = get_entry(acl, 0, ctypes.byref(entry))  # ACL_FIRST_ENTRY.
            while result == 0:
                tag = ctypes.c_int()
                if get_tag(entry, ctypes.byref(tag)) != 0:
                    error = ctypes.get_errno() or errno.EIO
                    raise OSError(error, os.strerror(error), "tool-root ancestor ACL")
                if tag.value != 2:  # Only ACL_EXTENDED_DENY cannot grant access.
                    return "allow"
                ctypes.set_errno(0)
                result = get_entry(acl, -1, ctypes.byref(entry))  # ACL_NEXT_ENTRY.
            if result != -1 or ctypes.get_errno() != errno.EINVAL:
                error = ctypes.get_errno() or errno.EIO
                raise OSError(error, os.strerror(error), "tool-root ancestor ACL")
            return "deny_only"
        finally:
            if free_acl(acl) != 0:
                error = ctypes.get_errno() or errno.EIO
                raise OSError(error, os.strerror(error), "tool-root ancestor ACL")
    error = ctypes.get_errno()
    if error == errno.ENOENT:
        return "none"
    error = error or errno.EIO
    raise OSError(error, os.strerror(error), "tool-root ancestor ACL")


def _checked_parent(parent):
    """Check the resolved destination parent and its ancestor permissions."""
    if not parent.is_dir() or parent.is_symlink():
        raise ValueError("destination parent must be an ordinary directory")
    before = os.stat(parent, follow_symlinks=False)
    resolved = parent.resolve(strict=True)
    after = os.stat(resolved, follow_symlinks=False)
    if (before.st_dev, before.st_ino) != (after.st_dev, after.st_ino):
        raise ValueError("destination parent changed during selection")
    # Establish each ancestor's protection before trusting a child path beneath it.
    for directory in reversed((resolved, *resolved.parents)):
        info = os.stat(directory, follow_symlinks=False)
        is_parent = directory == resolved
        directory_fd = os.open(directory, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        try:
            held = os.fstat(directory_fd)
            if (held.st_dev, held.st_ino) != (info.st_dev, info.st_ino):
                raise ValueError("destination ancestor changed during selection")
            mode = stat.S_IMODE(held.st_mode)
            if not stat.S_ISDIR(held.st_mode) or held.st_uid not in (0, os.geteuid()):
                raise ValueError("destination ancestor has an untrusted owner")
            if is_parent and (held.st_uid != os.geteuid() or mode & 0o077):
                raise ValueError("destination parent must be mode-private and locally owned")
            if mode & 0o022 and not mode & stat.S_ISVTX:
                raise ValueError("destination ancestor is writable without sticky protection")
            acl_status = _extended_acl_status(directory_fd)
            after_acl = os.fstat(directory_fd)
            if (after_acl.st_dev, after_acl.st_ino, after_acl.st_uid,
                    after_acl.st_mode, after_acl.st_ctime_ns) != (
                    held.st_dev, held.st_ino, held.st_uid,
                    held.st_mode, held.st_ctime_ns):
                raise ValueError("destination ancestor permissions changed during selection")
            if is_parent and acl_status != "none":
                raise ValueError("destination parent has an extended ACL")
            if acl_status == "allow":
                raise ValueError("destination ancestor ACL grants access")
        finally:
            os.close(directory_fd)
    return resolved


def prepare_clone(source, destination, expected):
    """Clone and inventory a root on one volume; return only on an exact match.

    `destination` must be absent under an ordinary, mode-private parent with
    trusted POSIX ancestors and no extended ACL on the parent. Ancestors may
    have deny-only ACLs but no allowing entries. This unready helper does not
    hold a no-writer lease after return. The clone is deleted on failure unless
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
    parent = _checked_parent(parent)
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
