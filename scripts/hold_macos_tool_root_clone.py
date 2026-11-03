"""Unready macOS retained-root candidate with append-only sealed clones.

The caller still owns release review, compiler-child lifetime and output
publication. A held selection can be shared by multiple builds, but it must
outlive every child that may resolve its paths.
"""

import ctypes
import errno
import json
import os
from pathlib import Path
import stat
import sys

if sys.platform == "darwin":
    import fcntl

from inventory_headless_tool_roots import inventory_root, inventory_root_with_witness
from prepare_macos_tool_root_clone import _checked_parent, prepare_clone

_FIELDS = {"sha256", "files", "symlinks", "directories", "logicalBytes"}
_POLICY = "macos-user-immutable-v1"
_MAX_RECORD_BYTES = 1024
_NOFOLLOW_FILE = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
_NOFOLLOW_DIR = _NOFOLLOW_FILE | getattr(os, "O_DIRECTORY", 0)


def _names(expected):
    if (not isinstance(expected, dict) or set(expected) != _FIELDS
            or not isinstance(expected["sha256"], str)
            or len(expected["sha256"]) != 64
            or any(char not in "0123456789abcdef" for char in expected["sha256"])
            or any(type(expected[key]) is not int or expected[key] < 0
                   for key in _FIELDS - {"sha256"})):
        raise ValueError("invalid expected tool-root inventory")
    digest = expected["sha256"]
    return f"root-{digest}", f".selection-{digest}.json", f".selection-{digest}.lock"


def _canonical_record(root_name, expected):
    record = {
        "buildReady": False,
        "format": 1,
        "inventory": expected,
        "rootName": root_name,
        "sealPolicy": _POLICY,
    }
    return (json.dumps(record, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
            + "\n").encode("ascii")


def _open_lock(parent, name, operation, *, create=False):
    lock_path = parent / name
    created = False
    if create:
        try:
            fd = os.open(lock_path, os.O_RDONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o400)
            created = True
        except FileExistsError:
            fd = os.open(lock_path, os.O_RDONLY | os.O_NOFOLLOW)
            created = False
    else:
        fd = os.open(lock_path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        os.set_inheritable(fd, False)
        info = os.fstat(fd)
        named = os.stat(lock_path, follow_symlinks=False)
        if (not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid()
                or info.st_nlink != 1 or info.st_size != 0
                or (info.st_dev, info.st_ino) != (named.st_dev, named.st_ino)):
            raise ValueError("retained-root lock has an untrusted identity")
        if not created and (stat.S_IMODE(info.st_mode) != 0o400
                            or not info.st_flags & stat.UF_IMMUTABLE):
            raise ValueError("retained-root lock lacks the permanent seal")
        fcntl.flock(fd, operation | fcntl.LOCK_NB)
        if created:
            os.fchmod(fd, 0o400)
            _fchflags(fd, os.fstat(fd).st_flags | stat.UF_IMMUTABLE)
            os.fsync(fd)
            info = os.fstat(fd)
        if stat.S_IMODE(info.st_mode) != 0o400 or not info.st_flags & stat.UF_IMMUTABLE:
            raise ValueError("retained-root lock lacks the permanent seal")
        named = os.stat(lock_path, follow_symlinks=False)
        if (info.st_dev, info.st_ino) != (named.st_dev, named.st_ino):
            raise ValueError("retained-root lock changed during acquisition")
        return fd
    except BaseException:
        os.close(fd)
        raise


def _fchflags(fd, flags):
    function = ctypes.CDLL(None, use_errno=True).fchflags
    function.argtypes = [ctypes.c_int, ctypes.c_uint]
    function.restype = ctypes.c_int
    if function(fd, flags) != 0:
        error = ctypes.get_errno() or errno.EIO
        raise OSError(error, os.strerror(error), "retained-root seal")


def _check_entry(info, is_directory):
    mode = stat.S_IMODE(info.st_mode)
    if (info.st_uid != os.geteuid() or mode & 0o7222
            or not info.st_flags & stat.UF_IMMUTABLE
            or (is_directory and mode & 0o500 != 0o500)
            or (not is_directory and info.st_nlink != 1)):
        raise ValueError("retained-root entry lacks the selected seal")


def _walk_seal(directory_fd, depth, apply):
    if depth > 128:
        raise ValueError("retained-root exceeds seal depth")
    with os.scandir(directory_fd) as entries:
        for entry in entries:
            before = os.stat(entry.name, dir_fd=directory_fd, follow_symlinks=False)
            if stat.S_ISDIR(before.st_mode):
                child_fd = os.open(entry.name, _NOFOLLOW_DIR, dir_fd=directory_fd)
                try:
                    held = os.fstat(child_fd)
                    if (held.st_dev, held.st_ino) != (before.st_dev, before.st_ino):
                        raise ValueError("retained-root directory changed")
                    _walk_seal(child_fd, depth + 1, apply)
                finally:
                    os.close(child_fd)
            elif stat.S_ISREG(before.st_mode):
                file_fd = os.open(entry.name, _NOFOLLOW_FILE, dir_fd=directory_fd)
                try:
                    held = os.fstat(file_fd)
                    if (held.st_dev, held.st_ino) != (before.st_dev, before.st_ino):
                        raise ValueError("retained-root file changed")
                    if apply:
                        _seal_fd(file_fd, False)
                    else:
                        _check_entry(os.fstat(file_fd), False)
                finally:
                    os.close(file_fd)
            elif not stat.S_ISLNK(before.st_mode):
                raise ValueError("unsupported retained-root entry")
            after = os.stat(entry.name, dir_fd=directory_fd, follow_symlinks=False)
            if (before.st_dev, before.st_ino) != (after.st_dev, after.st_ino):
                raise ValueError("retained-root entry changed during seal")
    if apply:
        _seal_fd(directory_fd, True)
    else:
        _check_entry(os.fstat(directory_fd), True)


def _seal_fd(fd, is_directory):
    info = os.fstat(fd)
    if info.st_uid != os.geteuid():
        raise ValueError("retained-root entry has an untrusted owner")
    os.fchmod(fd, stat.S_IMODE(info.st_mode) & 0o555)
    _fchflags(fd, os.fstat(fd).st_flags | stat.UF_IMMUTABLE)
    os.fsync(fd)
    _check_entry(os.fstat(fd), is_directory)


def _check_root(root, expected, expected_witnesses=None):
    root_fd = os.open(root, _NOFOLLOW_DIR)
    try:
        held = os.fstat(root_fd)
        named = os.stat(root, follow_symlinks=False)
        if (held.st_dev, held.st_ino) != (named.st_dev, named.st_ino):
            raise ValueError("retained-root path changed")
        _walk_seal(root_fd, 0, False)
        if expected_witnesses is None:
            observed, witnesses = inventory_root(root), None
        else:
            if not isinstance(expected_witnesses, dict) or not expected_witnesses:
                raise ValueError("retained-root witness selection is empty or invalid")
            observed, witnesses = inventory_root_with_witness(root, expected_witnesses)
        if observed != expected:
            raise ValueError("retained-root bytes differ from selected inventory")
        if expected_witnesses is not None and witnesses != expected_witnesses:
            raise ValueError("retained-root file witnesses differ from selected evidence")
        return root_fd, witnesses
    except BaseException:
        os.close(root_fd)
        raise


def prepare_sealed_clone(source, owner_parent, expected):
    """Cold candidate preparation; a failure leaves its digest name unusable."""
    if sys.platform != "darwin":
        raise ValueError("retained APFS tool-root cloning requires macOS")
    expected = expected.copy()
    root_name, record_name, lock_name = _names(expected)
    parent = _checked_parent(Path(owner_parent))
    lock_fd = _open_lock(parent, lock_name, fcntl.LOCK_EX, create=True)
    try:
        root = parent / root_name
        record = parent / record_name
        if root.exists() or root.is_symlink() or record.exists() or record.is_symlink():
            raise ValueError("retained-root selection already exists or is incomplete")
        prepare_clone(Path(source), root, expected)
        root_fd = os.open(root, _NOFOLLOW_DIR)
        try:
            _walk_seal(root_fd, 0, True)
        finally:
            os.close(root_fd)
        checked_fd, _ = _check_root(root, expected)
        os.close(checked_fd)
        record_bytes = _canonical_record(root_name, expected)
        temp_record = parent / f"{record_name}.pending"
        fd = os.open(temp_record, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        try:
            with os.fdopen(fd, "wb", closefd=False) as stream:
                stream.write(record_bytes)
                stream.flush()
            os.fsync(fd)
        finally:
            os.close(fd)
        os.rename(temp_record, record)
        record_fd = os.open(record, os.O_RDONLY | os.O_NOFOLLOW)
        try:
            _seal_fd(record_fd, False)
        finally:
            os.close(record_fd)
        parent_fd = os.open(parent, _NOFOLLOW_DIR)
        try:
            os.fsync(parent_fd)
        finally:
            os.close(parent_fd)
        return root, record
    finally:
        os.close(lock_fd)


class HeldToolRoot:
    """A cold-verified shared selection; caller owns every child lifetime."""

    def __init__(self, root, expected, root_fd, lock_fd, witnesses=None):
        self.root = root
        self.expected = expected.copy()
        self.witnesses = witnesses
        self._root_fd = root_fd
        self._lock_fd = lock_fd

    def assert_live(self):
        if self._root_fd is None:
            raise ValueError("retained-root lease is closed")
        held = os.fstat(self._root_fd)
        named = os.stat(self.root, follow_symlinks=False)
        if ((held.st_dev, held.st_ino) != (named.st_dev, named.st_ino)
                or not held.st_flags & stat.UF_IMMUTABLE):
            raise ValueError("retained-root lease lost its selected path or seal")

    def close(self):
        if self._root_fd is not None:
            try:
                os.close(self._root_fd)
            finally:
                os.close(self._lock_fd)
                self._root_fd = None
                self._lock_fd = None

    def __enter__(self):
        self.assert_live()
        return self

    def __exit__(self, _exc_type, _exc_value, _traceback):
        self.close()


def open_sealed_clone(owner_parent, expected, expected_witnesses=None):
    """Cold-verify one sealed selection before sharing it across warm builds."""
    if sys.platform != "darwin":
        raise ValueError("retained APFS tool-root selection requires macOS")
    expected = expected.copy()
    root_name, record_name, lock_name = _names(expected)
    parent = _checked_parent(Path(owner_parent))
    lock_fd = _open_lock(parent, lock_name, fcntl.LOCK_SH)
    try:
        record = parent / record_name
        fd = os.open(record, os.O_RDONLY | os.O_NOFOLLOW)
        try:
            info = os.fstat(fd)
            if (not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid()
                    or stat.S_IMODE(info.st_mode) != 0o400
                    or not info.st_flags & stat.UF_IMMUTABLE
                    or info.st_nlink != 1 or info.st_size > _MAX_RECORD_BYTES):
                raise ValueError("retained-root record has an untrusted identity")
            with os.fdopen(fd, "rb", closefd=False) as stream:
                raw = stream.read(_MAX_RECORD_BYTES + 1)
            if raw != _canonical_record(root_name, expected):
                raise ValueError("retained-root record differs from selected identity")
        finally:
            os.close(fd)
        root = parent / root_name
        root_fd, witnesses = _check_root(root, expected, expected_witnesses)
        return HeldToolRoot(root, expected, root_fd, lock_fd, witnesses)
    except BaseException:
        os.close(lock_fd)
        raise
