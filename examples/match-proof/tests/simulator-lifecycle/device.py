"""WP5.12: query CoreSimulator independently while the outer turn still owns admission."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts"))
import simulator_lifecycle as simulator

FIXTURE = Path(__file__).resolve()


def inventory():
    rows = json.loads(subprocess.check_output(
        ["xcrun", "simctl", "list", "devices", "-j"], text=True))["devices"]
    return [dict(name=device["name"], udid=device["udid"], state=device["state"], runtime=runtime)
            for runtime, devices in rows.items() for device in devices]


def all_shut(rows):
    assert all(device["state"] == "Shutdown" for device in rows), rows


def run_worker(case, marker):
    with simulator.supervise() as turn:
        first = inventory()
        calls = [turn.simctl("spawn", turn.udid, "launchctl", "managerpid") for _ in range(2)]
        assert calls[0].isdigit() and calls[0] == calls[1], calls
        second = inventory()
        for rows in (first, second):
            assert [device["udid"] for device in rows if device["state"] == "Booted"] == [turn.udid]
        marker.write_text(json.dumps(dict(udid=turn.udid, manager_pids=calls, before_calls=first, after_calls=second)))
        if case == "failure":
            turn.run([sys.executable, "-c", "raise SystemExit(23)"])
        if case == "cancellation":
            turn.run([sys.executable, "-c",
                      "from pathlib import Path; import sys, time; Path(sys.argv[1]).touch(); time.sleep(120)",
                      marker.with_suffix(".active")])


def wait_for(predicate, seconds):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.05)
    raise TimeoutError("Simulator proof worker did not reach its marker")


def proof(output):
    holder, _ = simulator.admission()
    assert holder, "run through the check's outer simulator lock"
    output.parent.mkdir(parents=True, exist_ok=True)
    attempt = output.with_name(f"device-attempt-{os.getpid()}.json")
    result = dict(commit=subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
                  lock_pid=holder["pid"], cases=[])
    (ROOT / "tmp").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(dir=ROOT / "tmp", prefix="simulator-proof-") as directory:
        for case, expected in (("success", 0), ("failure", 23), ("cancellation", 143)):
            before = inventory()
            all_shut(before)
            marker = Path(directory) / case
            env = dict(os.environ, **{simulator.RECORD_ENV: str(marker.with_suffix(".jsonl"))})
            with subprocess.Popen([sys.executable, FIXTURE, case, marker], env=env,
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                  text=True, start_new_session=True) as worker:
                if case == "cancellation":
                    try:
                        wait_for(marker.with_suffix(".active").is_file, 160)
                    finally:
                        worker.terminate()
                stdout, stderr = worker.communicate(timeout=180)
                # Query now, before the outer runner can release its lock.
                after = inventory()
                during = json.loads(marker.read_text()) if marker.exists() else {}
                events_path = marker.with_suffix(".jsonl")
                events = [json.loads(line) for line in events_path.read_text().splitlines()] if events_path.exists() else []
                result["cases"].append(dict(case=case, result=worker.returncode,
                                            before=before, during=during, after=after,
                                            lifecycle=events, stderr=stderr))
                attempt.write_text(json.dumps(result, indent=2) + "\n")
                assert worker.returncode == expected, (case, worker.returncode, stdout, stderr)
                all_shut(after)
                assert simulator.admission()[0]["pid"] == holder["pid"]
                assert any(device["udid"] == during["udid"] for device in after)
                assert events[-1]["event"] == "shutdown_verified"
    result["pass"] = True
    attempt.write_text(json.dumps(result, indent=2) + "\n")
    attempt.replace(output)
    print(output)


if __name__ == "__main__":
    if len(sys.argv) == 3:
        try:
            run_worker(sys.argv[1], Path(sys.argv[2]))
        except subprocess.CalledProcessError as error:
            raise SystemExit(error.returncode) from None
    else:
        proof(ROOT / ".orchestrate/evidence/SESSION/WP5/simulator-lifecycle/device.json")
