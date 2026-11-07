"""Hold an unready release-record bundle under a sealed macOS owner.

The caller supplies the reviewed manifest digest. This module matches local
records and retains their selected bytes; it does not authenticate acquisition
or turn a candidate into a ready BuildRevision.
"""

import hashlib
import os
from pathlib import Path
import shutil
import stat
import sys
import tempfile

from hold_macos_tool_root_clone import _check_entry, open_sealed_clone, prepare_sealed_clone
from inventory_headless_tool_roots import inventory_root
from prepare_macos_tool_root_clone import _checked_parent
from verify_headless_release_inputs import (
    bounded_regular_bytes, canonical_json_bytes, load_release_selection,
    validate_manifest, validate_origin_evidence,
)


_MANIFEST_NAME = "!manifest.json"
_MAX_MANIFEST = 64 * 1024
_MAX_REPORT = 512 * 1024
_NOFOLLOW_FILE = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)


def _selected_record_bytes(manifest_path, expected_sha256):
    raw = bounded_regular_bytes(manifest_path, _MAX_MANIFEST)
    manifest, actual_sha256 = canonical_json_bytes(raw, manifest_path)
    validate_manifest(manifest)
    if actual_sha256 != expected_sha256:
        raise ValueError("release manifest differs from selected reviewed digest")
    evidence_name = manifest["originEvidence"]["file"]
    evidence_path = manifest_path.parent / evidence_name
    evidence_raw = bounded_regular_bytes(evidence_path, _MAX_MANIFEST)
    if hashlib.sha256(evidence_raw).hexdigest() != manifest["originEvidence"]["sha256"]:
        raise ValueError("release origin record differs from selected manifest")
    evidence, _ = canonical_json_bytes(evidence_raw, evidence_path)
    validate_origin_evidence(evidence)
    selected = {_MANIFEST_NAME: raw, evidence_name: evidence_raw}
    for proof in evidence["proofs"].values():
        name = proof["file"]
        if name in selected:
            raise ValueError("release proof aliases another selected record")
        path = manifest_path.parent / name
        proof_raw = bounded_regular_bytes(path, _MAX_REPORT)
        if hashlib.sha256(proof_raw).hexdigest() != proof["sha256"]:
            raise ValueError(f"release proof differs from selected record: {name}")
        canonical_json_bytes(proof_raw, path)
        selected[name] = proof_raw
    return selected


def prepare_sealed_release_records(manifest_path, expected_sha256, source,
                                   cache_manifest_path, target, owner_parent):
    """Clone exact selected records; a failed digest name is never repaired."""
    if sys.platform != "darwin":
        raise ValueError("held release records require macOS")
    owner = _checked_parent(Path(owner_parent))
    records = _selected_record_bytes(Path(manifest_path), expected_sha256)
    stage = Path(tempfile.mkdtemp(prefix=".release-stage-", dir=owner))
    try:
        for name, raw in records.items():
            path = stage / name
            with path.open("xb") as output:
                output.write(raw)
                output.flush()
                os.fsync(output.fileno())
        _, observed_sha, _, _, _ = load_release_selection(
            stage / _MANIFEST_NAME, source, cache_manifest_path, target,
        )
        if observed_sha != expected_sha256:
            raise ValueError("staged release manifest changed")
        tree = inventory_root(stage)
        root, _ = prepare_sealed_clone(stage, owner, tree)
        return root, tree
    finally:
        shutil.rmtree(stage)


class HeldReleaseRecords:
    def __init__(self, lease, files, manifest_sha256, manifest, candidate,
                 cache_manifest_sha256, rust_witnesses):
        self.root = lease.root
        self.tree = lease.expected
        self.manifest_sha256 = manifest_sha256
        self.manifest = manifest
        self.candidate = candidate
        self.cache_manifest_sha256 = cache_manifest_sha256
        self.rust_witnesses = rust_witnesses
        self._lease = lease
        self._files = files

    def assert_live(self):
        self._lease.assert_live()
        for path, fd in self._files:
            held = os.fstat(fd)
            named = os.stat(path, follow_symlinks=False)
            _check_entry(held, False)
            if (held.st_dev, held.st_ino) != (named.st_dev, named.st_ino):
                raise ValueError("held release record changed identity")

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
            self._lease.close()
        if first_error is not None:
            raise first_error

    def __enter__(self):
        self.assert_live()
        return self

    def __exit__(self, _exc_type, _exc_value, _traceback):
        self.close()


def open_sealed_release_records(owner_parent, tree, expected_sha256, source,
                                cache_manifest_path, target):
    """Cold-check exact record bytes under a shared sealed-root lease."""
    lease = open_sealed_clone(owner_parent, tree)
    files = []
    try:
        manifest_path = lease.root / _MANIFEST_NAME
        selected = _selected_record_bytes(manifest_path, expected_sha256)
        names = {entry.name for entry in os.scandir(lease.root)}
        if names != set(selected):
            raise ValueError("held release record membership differs")
        manifest, actual_sha, candidate, cache_manifest_sha, rust_witnesses = load_release_selection(
            manifest_path, source, cache_manifest_path, target,
        )
        if actual_sha != expected_sha256:
            raise ValueError("held release manifest differs from selected digest")
        for name in sorted(selected):
            path = lease.root / name
            fd = os.open(path, _NOFOLLOW_FILE)
            files.append((path, fd))
            info = os.fstat(fd)
            if not stat.S_ISREG(info.st_mode):
                raise ValueError("held release record is not ordinary")
            _check_entry(info, False)
            named = os.stat(path, follow_symlinks=False)
            if (info.st_dev, info.st_ino) != (named.st_dev, named.st_ino):
                raise ValueError("held release record changed identity")
        selection = HeldReleaseRecords(
            lease, files, actual_sha, manifest, candidate, cache_manifest_sha, rust_witnesses,
        )
        selection.assert_live()
        return selection
    except BaseException:
        for _, fd in files:
            try:
                os.close(fd)
            except OSError:
                pass
        try:
            lease.close()
        except OSError:
            pass
        raise
