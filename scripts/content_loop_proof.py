#!/usr/bin/env python3
"""Run one CONTENT-LOOP target proof with its exact clean source revision in the log."""
import json
import subprocess
import sys


def main():
    command = sys.argv[1:]
    if command[:1] == ["--"]:
        command = command[1:]
    if not command:
        raise SystemExit("usage: content_loop_proof.py -- COMMAND...")
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    dirty = subprocess.check_output(["git", "status", "--porcelain"], text=True).strip()
    if dirty:
        raise SystemExit("proof requires a committed source tree")
    print(json.dumps({"revision": revision, "command": command}), flush=True)
    result = subprocess.run(command, check=False)
    print(json.dumps({"revision": revision, "exit": result.returncode}), flush=True)
    raise SystemExit(result.returncode)


if __name__ == "__main__":
    main()
