"""Candidate supervisor for one selected macOS sandboxed Cargo build.

The caller owns the request and output directory. This is not a release-ready
compiler adapter or a substitute for a reviewed native environment.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import stat
import sys

from prepare_headless_cargo_home import collect_bounded
from verify_rustc_receipts import MAX_RECEIPT, MAX_RECEIPTS, NAME, parse_receipt

MAX_REQUEST = 64 * 1024
MAX_STDOUT = 4 * 1024 * 1024
MAX_STDERR = 1024 * 1024


def ordinary_file(path, maximum):
    def identity(info):
        return (info.st_dev, info.st_ino, info.st_mode, info.st_size,
                info.st_mtime_ns, info.st_ctime_ns)

    before = path.lstat()
    if not stat.S_ISREG(before.st_mode) or before.st_size > maximum:
        raise ValueError(f"missing, aliased or oversized supervisor file: {path}")
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), "rb") as stream:
        if identity(os.fstat(stream.fileno())) != identity(before):
            raise ValueError(f"supervisor file changed before read: {path}")
        raw = stream.read(maximum + 1)
        if identity(os.fstat(stream.fileno())) != identity(before):
            raise ValueError(f"supervisor file changed during read: {path}")
    if len(raw) != before.st_size or identity(path.lstat()) != identity(before):
        raise ValueError(f"supervisor file changed: {path}")
    return raw


def validate_request(request):
    if not isinstance(request, dict) or set(request) != {
        "format", "command", "cwd", "environment", "output", "ownerPid",
    } or request["format"] != 1:
        raise ValueError("unsupported native supervisor request")
    if (not isinstance(request["ownerPid"], int) or isinstance(request["ownerPid"], bool)
            or request["ownerPid"] <= 1):
        raise ValueError("native supervisor owner PID is invalid")
    command = request["command"]
    if (not isinstance(command, list) or not 2 <= len(command) <= 64
            or command[0] != "/usr/bin/sandbox-exec"
            or any(not isinstance(arg, str) or not arg or len(arg) > 4096 or "\0" in arg
                   for arg in command)):
        raise ValueError("native supervisor requires one bounded sandbox command")
    environment = request["environment"]
    if (not isinstance(environment, dict) or len(environment) > 64
            or any(not isinstance(key, str) or not re.fullmatch(r"[A-Z][A-Z0-9_]*", key)
                   or not isinstance(value, str) or len(value) > 8192 or "\0" in value
                   for key, value in environment.items())):
        raise ValueError("native supervisor environment is malformed")
    cwd = Path(request["cwd"])
    output = Path(request["output"])
    if (not cwd.is_absolute() or cwd.is_symlink() or not cwd.is_dir()
            or not output.is_absolute() or output.is_symlink() or not output.is_dir()
            or cwd == output):
        raise ValueError("native supervisor roots are not ordinary absolute directories")
    return command, cwd, environment, output


def write_json(path, value):
    with path.open("xb") as stream:
        stream.write(json.dumps(value, sort_keys=True, separators=(",", ":")).encode() + b"\n")


def observed_receipts(output):
    """Inventory stopped-job receipts; this cannot prove every child was seen."""
    directory = output / "rustc-receipts"
    if not directory.exists() and not directory.is_symlink():
        return {"kind": "absent"}
    metadata = directory.lstat()
    if not stat.S_ISDIR(metadata.st_mode):
        raise ValueError("native receipt directory is aliased or irregular")
    entries = []
    with os.scandir(directory) as scan:
        for entry in scan:
            if len(entries) == MAX_RECEIPTS:
                raise ValueError("interrupted native receipt set exceeds limit")
            entries.append(Path(entry.path))
    entries.sort()
    digest = hashlib.sha256(b"thrum.interrupted-receipts.v1\0")
    counts = {"completedSuccess": 0, "completedNonzero": 0,
              "pending": 0, "unparsed": 0}
    total_bytes = 0
    for path in entries:
        if not NAME.fullmatch(path.name):
            raise ValueError("unexpected interrupted native receipt entry")
        raw = ordinary_file(path, MAX_RECEIPT)
        total_bytes += len(raw)
        try:
            parsed = parse_receipt(raw, allow_pending=True)
        except ValueError:
            state = "unparsed"
        else:
            if parsed.get("pending"):
                state = "pending"
            elif parsed["outcome"] == 0 and parsed["status"] == 0:
                state = "completedSuccess"
            else:
                state = "completedNonzero"
        counts[state] += 1
        digest.update(path.name.encode("ascii") + b"\0")
        digest.update(len(raw).to_bytes(8, "little"))
        digest.update(hashlib.sha256(raw).digest())
        digest.update(state.encode("ascii") + b"\0")
    return {"kind": "observedOnly", "files": len(entries), "bytes": total_bytes,
            "setSha256": digest.hexdigest(), **counts}


def fault_report(output, kind, detail):
    try:
        inventory = observed_receipts(output)
    except (OSError, ValueError) as error:
        inventory = {"kind": "unavailable", "detail": str(error)[:256]}
    write_json(output / "cargo-supervisor.json", {
        "format": 1, "kind": kind, "buildReady": False,
        "detail": detail[:512], "receiptInventory": inventory,
    })


def run(request_path, timeout_seconds=120):
    if not request_path.is_absolute() or request_path.is_symlink():
        raise ValueError("native supervisor request must be an ordinary absolute file")
    request = json.loads(ordinary_file(request_path, MAX_REQUEST))
    command, cwd, environment, output = validate_request(request)
    if os.getppid() != request["ownerPid"]:
        raise ValueError("native supervisor lost its launching owner")
    if request_path.parent != output:
        raise ValueError("native supervisor request escapes selected output")
    stdout_path = output / "cargo.stdout"
    stderr_path = output / "cargo.stderr"
    report_path = output / "cargo-supervisor.json"
    if any(path.exists() or path.is_symlink()
           for path in (stdout_path, stderr_path, report_path)):
        raise ValueError("native supervisor output already exists")

    def interrupted(_signal, _frame):
        raise KeyboardInterrupt("native build supervisor interrupted")

    previous_term = signal.signal(signal.SIGTERM, interrupted)
    try:
        status, stdout, stderr = collect_bounded(
            command, cwd, environment, stdout_limit=MAX_STDOUT,
            stderr_limit=MAX_STDERR, timeout_seconds=timeout_seconds,
            parent_pid=request["ownerPid"],
            label="Cargo build",
        )
    except KeyboardInterrupt as error:
        fault_report(output, "interrupted", str(error))
        return 125
    except Exception as error:
        fault_report(output, "supervisorError", str(error))
        return 125
    finally:
        signal.signal(signal.SIGTERM, previous_term)
    with stdout_path.open("xb") as stream:
        stream.write(stdout)
    with stderr_path.open("xb") as stream:
        stream.write(stderr)
    write_json(report_path, {
        "format": 1,
        "kind": "completed" if status in (0, 101) else "refusedStatus",
        "buildReady": False,
        "cargoStatus": status,
        "stdoutBytes": len(stdout), "stdoutSha256": hashlib.sha256(stdout).hexdigest(),
        "stderrBytes": len(stderr), "stderrSha256": hashlib.sha256(stderr).hexdigest(),
    })
    return status if status in (0, 101) else 125


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--request", required=True, type=Path)
    args = parser.parse_args()
    return run(args.request)


if __name__ == "__main__":
    sys.exit(main())
