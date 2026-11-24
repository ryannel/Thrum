#!/usr/bin/env -S python3 -S
"""Fakes used only by the isolated outer-runner proof."""
import json
import os
from pathlib import Path
import sys
import time

case = Path(os.environ["THRUM_SIMULATOR_PROOF_CASE"])
statefile = case / "state"
events = case / "events.jsonl"


def event(name, **fields):
    with events.open("a") as stream:
        stream.write(json.dumps(dict(event=name, at=time.monotonic(), pid=os.getpid(), **fields)) + "\n")


def read_state():
    state, *data = statefile.read_text().split()
    if state == "pending" and time.monotonic() >= float(data[0]):
        state = "Shutdown"
        statefile.write_text(state)
    return "Shutting Down" if state == "pending" else state


if Path(sys.argv[0]).name == "bash":
    route, *arguments = sys.argv[1:]
    if route.endswith("check-simulator.sh"):
        if arguments == ["--prepare"]:
            event("prepared")
            sys.exit(0)
        assert arguments == ["--device-worker"] and os.environ["THRUM_PROOF_SIMULATOR"] == "owned"
        event("worker_active")
        (case / "active").touch()
        time.sleep(120)
        sys.exit(2)
    os.execv("/bin/bash", ["bash", *sys.argv[1:]])

_, tool, operation, *arguments = sys.argv
assert tool == "simctl"
if operation == "list":
    state = read_state()
    if state == "Shutdown" and (case / "shutdown_started").exists():
        event("shutdown_seen")
    print(json.dumps({"devices": {"com.apple.CoreSimulator.SimRuntime.iOS-27-2": [{
        "name": "Thrum proof iPhone 18 Pro", "udid": "owned", "state": state,
        "deviceTypeIdentifier": "com.apple.CoreSimulator.SimDeviceType.iPhone-18-Pro",
    }]}}))
else:
    assert arguments[0] == "owned"
    if operation == "bootstatus":
        statefile.write_text("Booted")
        event("booted")
    elif operation == "shutdown" and read_state() == "Booted":
        (case / "shutdown_started").touch()
        statefile.write_text(f"pending {time.monotonic() + 0.65}")
        event("shutdown_requested")
    else:
        assert operation in ("erase", "shutdown"), operation
