"""Portable identity of one selected private Cargo release-input candidate."""

import hashlib
import json


DOMAIN = b"thrum.release-cargo-selection.v1\0"


def selection_sha256(candidate):
    selected = {
        "target": candidate["target"],
        "lockSha256": candidate["lockSha256"],
        "privateRoot": candidate["privateRoot"],
        "registryArchives": candidate["registryArchives"],
        "registryIndexRecords": candidate["registryIndexRecords"],
    }
    encoded = json.dumps(
        selected, sort_keys=True, separators=(",", ":"), ensure_ascii=True,
        allow_nan=False,
    ).encode("utf-8")
    return hashlib.sha256(DOMAIN + encoded).hexdigest()
