"""Own one simulator turn from admission through fresh shutdown verification."""

import argparse
from contextlib import contextmanager
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
NAME = "Thrum proof iPhone 18 Pro"
DEVICE_TYPE = "com.apple.CoreSimulator.SimDeviceType.iPhone-18-Pro"
RUNTIME = "com.apple.CoreSimulator.SimRuntime.iOS-27-2"
DEVICE_ENV = "THRUM_PROOF_SIMULATOR"
RECORD_ENV = "THRUM_SIMULATOR_LIFECYCLE_RECORD"
HOLD_SECONDS = 1800
CLEANUP_SECONDS = 25
CANCEL_GRACE_SECONDS = 9
SIGNALS = tuple(getattr(signal, name) for name in ("SIGTERM", "SIGINT", "SIGHUP") if hasattr(signal, name))


def orchestration_script():
    script = os.environ.get("ORCHESTRATE_SCRIPT")
    if not script:
        raise RuntimeError("Set ORCHESTRATE_SCRIPT to the installed agent_loop.py")
    return script


def ancestor_locks():
    report = json.loads(subprocess.check_output(
        [sys.executable, orchestration_script(), "locks", "--worktree", ROOT, "--json"],
        text=True, timeout=5))
    command = ["ps", "-axo", "pid=,ppid="]
    try:
        processes = subprocess.check_output(command, text=True, timeout=3)
    except subprocess.TimeoutExpired:
        # Simulator startup can delay a process-table walk on a busy host.
        processes = subprocess.check_output(command, text=True, timeout=12)
    parents = {int(pid): int(parent) for pid, parent in (line.split() for line in processes.splitlines())}
    ancestors = set()
    parent = os.getppid()
    while parent > 1 and parent not in ancestors:
        ancestors.add(parent)
        parent = parents.get(parent, 0)
    held = {resource["lock"]: holder for resource in report["locks"]
            for holder in resource["holders"] if holder["pid"] in ancestors}
    return held, report.get("hold_limit_s") or HOLD_SECONDS


def admission():
    held, limit = ancestor_locks()
    if "gpu" in held:
        raise RuntimeError("Release the GPU lock before taking a simulator turn")
    return held.get("simulator"), min(limit, HOLD_SECONDS)


def lock_command(argv):
    return [sys.executable, orchestration_script(), "lock", "simulator", "--worktree", str(ROOT),
            "--", *map(str, argv)]


def exec_with_admission(argv, *, independent=False):
    """Used by an entry point whose original PID must become the supervisor."""
    holder, _ = admission()
    if holder and independent:
        raise RuntimeError("This command needs separate simulator turns; run without an outer lock")
    command = list(map(str, argv)) if holder else lock_command(argv)
    os.execv(command[0], command)


def run_case(argv, *, record=None, independent=True):
    """Run a standalone device case and return its stdout after verified shutdown."""
    holder, _ = admission()
    if holder and independent:
        raise RuntimeError("This command needs separate simulator turns; run without an outer lock")
    command = [sys.executable, __file__, "run", "--", *map(str, argv)]
    if not holder:
        command = lock_command(command)
    env = dict(os.environ)
    if record:
        env[RECORD_ENV] = str(record)
    result = subprocess.check_output(command, cwd=ROOT, env=env, text=True)
    if record and not holder:
        with open(record, "a") as stream:
            stream.write(json.dumps(dict(event="lock_released", utc_s=time.time())) + "\n")
    return result


class Cancelled(BaseException):
    def __init__(self, signum):
        self.signum = signum


def stop_worker(process, *, grace=0.35):
    """Stop the entire worker group, including descendants of an exited leader."""
    for sig in (signal.SIGTERM, signal.SIGKILL):
        # Darwin can report EPERM for the unreaped group leader after TERM.
        process.poll()
        try:
            os.killpg(process.pid, sig)
        except ProcessLookupError:
            break
        if sig == signal.SIGTERM:
            time.sleep(grace)
    process.wait(timeout=1)


class Turn:
    def __init__(self, holder, deadline, *, clock=time.monotonic):
        self.holder = holder
        self.deadline = deadline
        self.clock = clock
        self.cancelled = None
        self.cancelled_at = None
        self.udid = None
        self.selecting = False

    def event(self, name, **fields):
        row = dict(event=name, utc_s=time.time(), pid=os.getpid(), lock_pid=self.holder["pid"],
                   udid=self.udid, **fields)
        line = json.dumps(row)
        print(line, file=sys.stderr, flush=True)
        if path := os.environ.get(RECORD_ENV):
            with open(path, "a") as stream:
                stream.write(line + "\n")

    def signal(self, signum, _frame):
        if self.cancelled is None:
            self.cancelled = signum
            self.cancelled_at = time.monotonic()

    def check(self):
        if self.cancelled is not None:
            raise Cancelled(self.cancelled)
        if self.clock() >= self.deadline:
            self.event("worker_deadline")
            raise TimeoutError("Simulator worker deadline expired before cleanup reserve")

    def run(self, argv, *, capture=False, env=None, check=True):
        self.check()
        argv = list(map(str, argv))
        with subprocess.Popen(argv, cwd=ROOT, env=env, text=True, start_new_session=True,
                              stdout=subprocess.PIPE if capture else None,
                              stderr=subprocess.PIPE if capture else None) as process:
            try:
                while True:
                    self.check()
                    try:
                        out, err = process.communicate(timeout=0.1)
                        break
                    except subprocess.TimeoutExpired:
                        continue
            finally:
                stop_worker(process)
        result = subprocess.CompletedProcess(argv, process.returncode, out, err)
        if check:
            result.check_returncode()
        return result

    def simctl(self, *args):
        return self.run(["xcrun", "simctl", *args], capture=True).stdout.strip()

    @staticmethod
    def owned(inventory):
        matches = [device for device in inventory["devices"].get(RUNTIME, [])
                   if device["name"] == NAME and device.get("deviceTypeIdentifier") == DEVICE_TYPE]
        if len(matches) > 1:
            raise RuntimeError("Multiple simulators have the reserved Thrum name and type")
        return matches[0] if matches else None

    def select(self, requested):
        device = self.owned(json.loads(self.simctl("list", "devices", "-j")))
        if requested is not None and (device is None or requested != device["udid"]):
            raise RuntimeError(f"Simulator {requested} is outside this proof's ownership")
        if device:
            self.udid = device["udid"]
            if device["state"] != "Shutdown":
                self.event("recover_stranded", state=device["state"])
                self.shutdown()
        else:
            self.selecting = True
            self.udid = self.simctl("create", NAME, DEVICE_TYPE, RUNTIME)
            self.selecting = False

    def boot(self, fresh):
        if fresh:
            # Erase only the owned, shut-down device. This clears all persisted
            # app containers, runner installations and XCTest service state.
            self.simctl("erase", self.udid)
        self.run(["xcrun", "simctl", "bootstatus", self.udid, "-b"], capture=True)
        self.event("booted", fresh=fresh)

    def request_shutdown(self):
        return subprocess.Popen(["xcrun", "simctl", "shutdown", self.udid],
                                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    def shutdown(self):
        # Use the real clock for teardown even when a proof injects the worker clock.
        deadline = time.monotonic() + CLEANUP_SECONDS
        last = "unknown"
        diagnostic = None
        request = None

        def remaining():
            limit = deadline
            if self.cancelled_at is not None:
                limit = min(limit, self.cancelled_at + CANCEL_GRACE_SECONDS)
            return limit - time.monotonic()

        if self.udid is None and not self.selecting:
            return
        try:
            while remaining() > 0:
                try:
                    # Keep the shutdown request alive while independent queries
                    # check CoreSimulator. Killing it during startup can leave
                    # the daemon with no request at all.
                    if self.udid and (request is None or request.poll() not in (None, 0)):
                        request = self.request_shutdown()
                    result = subprocess.run(
                        ["xcrun", "simctl", "list", "devices", "-j"], capture_output=True,
                        text=True, timeout=min(2.0, max(0.01, remaining())))
                    if result.returncode:
                        diagnostic = result.stderr.strip()
                    else:
                        device = self.owned(json.loads(result.stdout))
                        if self.selecting and not self.udid:
                            if device is None:
                                return
                            self.udid = device["udid"]
                        last = device["state"] if device and device["udid"] == self.udid else "missing"
                        if last == "Shutdown":
                            self.event("shutdown_verified", state=last)
                            return
                except (subprocess.SubprocessError, OSError, ValueError) as error:
                    diagnostic = str(error)
                time.sleep(0.1)
        finally:
            if request is not None and request.poll() is None:
                request.kill()
                request.wait(timeout=0.3)
        self.event("shutdown_unverified", state=last, diagnostic=diagnostic)
        raise RuntimeError(f"Simulator {self.udid} shutdown unverified; last state: {last}; "
                           f"query error: {diagnostic}")

    def worker_env(self, env=None):
        return dict(os.environ if env is None else env, **{DEVICE_ENV: self.udid})


@contextmanager
def supervise(*, requested=None, fresh=False, seconds=HOLD_SECONDS, clock=time.monotonic):
    started = clock()
    holder, limit = admission()
    if holder is None:
        raise RuntimeError("Simulator supervisor requires an ancestor holding simulator admission")
    remaining = min(seconds, limit - holder["held_s"])
    turn = Turn(holder, started + remaining - CLEANUP_SECONDS - 2, clock=clock)
    handlers = {sig: signal.signal(sig, turn.signal) for sig in SIGNALS}
    try:
        turn.event("admitted", remaining_s=remaining)
        turn.select(requested)
        turn.boot(fresh)
        yield turn
        turn.check()
    except Cancelled as error:
        turn.event("worker_cancelled", signal=error.signum)
        raise SystemExit(128 + error.signum) from None
    finally:
        try:
            turn.shutdown()
        finally:
            for sig, previous in handlers.items():
                signal.signal(sig, previous)
    if turn.cancelled is not None:
        turn.event("worker_cancelled", signal=turn.cancelled)
        raise SystemExit(128 + turn.cancelled)


def worker_device(*, attributed=False):
    """Validate the owned UDID passed to a worker before it touches an app."""
    if not attributed and admission()[0] is None:
        raise RuntimeError("A simulator worker needs its supervisor's admission")
    udid = os.environ[DEVICE_ENV]
    inventory = json.loads(subprocess.check_output(
        ["xcrun", "simctl", "list", "devices", "-j"], text=True, timeout=5))
    device = Turn.owned(inventory)
    if device is None or device["udid"] != udid or device["state"] != "Booted":
        raise RuntimeError(f"Simulator {udid} is outside this proof's booted device ownership")
    return udid


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["run", "spawn", "fresh", "worker-device"])
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.action == "worker-device":
        print(worker_device())
        return
    command = args.command
    if command[:1] == ["--"]:
        command = command[1:]
    with supervise(fresh=args.action == "fresh") as turn:
        if args.action == "run":
            command = ["xcrun", "simctl", "spawn", turn.udid, *command]
        result = turn.run(command, env=turn.worker_env(), check=False)
        if result.returncode:
            raise SystemExit(result.returncode if result.returncode > 0 else 128 - result.returncode)


if __name__ == "__main__":
    main()
