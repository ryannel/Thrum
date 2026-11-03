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


def run(request_path):
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
            stderr_limit=MAX_STDERR, parent_pid=request["ownerPid"],
            label="Cargo build",
        )
    except Exception as error:
        write_json(report_path, {
            "format": 1, "kind": "supervisorError", "detail": str(error),
        })
        return 125
    finally:
        signal.signal(signal.SIGTERM, previous_term)
    if status not in (0, 101):
        write_json(report_path, {
            "format": 1, "kind": "refusedStatus", "cargoStatus": status,
        })
        return 125
    with stdout_path.open("xb") as stream:
        stream.write(stdout)
    with stderr_path.open("xb") as stream:
        stream.write(stderr)
    write_json(report_path, {
        "format": 1, "kind": "completed", "cargoStatus": status,
        "stdoutBytes": len(stdout), "stdoutSha256": hashlib.sha256(stdout).hexdigest(),
        "stderrBytes": len(stderr), "stderrSha256": hashlib.sha256(stderr).hexdigest(),
    })
    return status


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--request", required=True, type=Path)
    args = parser.parse_args()
    return run(args.request)


if __name__ == "__main__":
    sys.exit(main())
