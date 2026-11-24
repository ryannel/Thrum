"""WP5.14: qualify the installed runner with production entry points in an isolated repo."""
import importlib.util
import json
import os
from pathlib import Path
import shlex
import shutil
import signal
import subprocess
import sys
import tempfile
import time

FIXTURE = Path(__file__).resolve().parent
ROOT = FIXTURE.parents[3]
SCRIPT = Path(os.environ["ORCHESTRATE_SCRIPT"]).resolve()


def run_check(tree, command, result):
    spec = importlib.util.spec_from_file_location("orchestrate_runner", SCRIPT)
    runner = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = runner
    spec.loader.exec_module(runner)
    runner.install_stop()
    report = runner.run_check(Path(tree), command, "simulator", 70, save=False)
    Path(result).write_text(json.dumps(dict(report, at=time.monotonic(), pid=os.getpid())))


def wait_for(check, label):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if check():
            return
        time.sleep(0.05)
    raise TimeoutError(label)


def declared(module, item):
    data = json.loads((ROOT / f"docs/plans/{module}/DONE.json").read_text())
    return next(row["check"] for package in data["work_packages"] for row in package["items"]
                if row["id"] == item)


def setup(tree):
    subprocess.run(["git", "init", "-q", tree], check=True)
    shutil.copy2(ROOT / "ORCHESTRATE.md", tree)
    # Use the checkout's production Python modules so new imports in either
    # entry point also resolve inside the isolated runner.
    scripts = subprocess.check_output(
        ["git", "ls-files", "-z", "--", "scripts/*.py"], cwd=ROOT, text=True)
    for name in scripts.rstrip("\0").split("\0"):
        destination = tree / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / name, destination)
    sample_slots = subprocess.check_output(
        ["git", "grep", "-l", "const SAMPLE_" "SLOTS: usize", "--", "crates", "examples"],
        cwd=ROOT, text=True).splitlines()
    for name in sample_slots:
        (tree / name).parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / name, tree / name)
    subprocess.run(["git", "add", "."], cwd=tree, check=True)
    subprocess.run(["git", "-c", "user.name=Simulator proof",
                    "-c", "user.email=simulator-proof@example.invalid",
                    "-c", "commit.gpgsign=false", "commit", "-qm", "Isolated proof inputs"],
                   cwd=tree, check=True)
    shims = tree / "shims"
    shims.mkdir()
    for name in ("bash", "xcrun"):
        shutil.copy2(FIXTURE / "fake_command.py", shims / name)
        (shims / name).chmod(0o755)
    # Inject only the worker clock. Cleanup and the installed runner retain real time.
    (shims / "sitecustomize.py").write_text(
        "import functools, os, sys, time\n"
        "from pathlib import Path\n"
        "if sys.argv[0].endswith('apple_presentation.py') and (case := os.environ.get('THRUM_SIMULATOR_PROOF_CASE')):\n"
        "    import v1_simulator_route as route\n"
        "    def clock():\n"
        "        return time.monotonic() + (1500 if (Path(case) / 'expire').exists() else 0)\n"
        "    route.presentation = functools.partial(route.presentation, clock=clock)\n")
    return shims


def proof_case(tree, shims, name, command, *, timer=False, control=False):
    case = tree / name
    case.mkdir()
    (case / "state").write_text("Shutdown")
    env = dict(os.environ, THRUM_SIMULATOR_PROOF_CASE=str(case),
               PATH=f"{shims}{os.pathsep}{os.environ['PATH']}",
               PYTHONPATH=f"{shims}{os.pathsep}{tree / 'scripts'}")
    first = case / "first.json"
    if control:
        # An outer shell that returns as soon as it receives TERM instead of
        # letting the PID become the supervisor. The child still gets TERM.
        command = f"{command.removeprefix('exec ')} & trap 'exit 130' TERM; wait"
    launch = [sys.executable, __file__, "run"]
    with subprocess.Popen([*launch, tree, command, first], cwd=tree, env=env) as holder:
        wait_for(lambda: (case / "active").exists() or first.exists(),
                 "first check never started the route worker")
        if not (case / "active").exists():
            raise RuntimeError(first.read_text())
        contender = case / "contender.json"
        observe = (
            "from pathlib import Path; import json, time; "
            f"state = Path({str(case / 'state')!r}).read_text(); "
            f"Path({str(contender)!r}).write_text(json.dumps(dict(state=state, at=time.monotonic())))"
        )
        second = case / "second.json"
        observe_command = shlex.join(["exec", sys.executable, "-c", observe])
        with subprocess.Popen([*launch, tree, observe_command, second], cwd=tree, env=env) as queued:
            def waiting():
                report = json.loads(subprocess.check_output([sys.executable, SCRIPT, "locks",
                                      "--worktree", tree, "--json"], cwd=tree, env=env, text=True))
                return any(entry["pid"] == queued.pid for lock in report["locks"]
                           if lock["lock"] == "simulator" for entry in lock["queued"])

            wait_for(waiting, "second runner was not queued")
            assert not contender.exists()
            triggered = time.monotonic()
            if timer:
                (case / "expire").touch()
            else:
                holder.send_signal(signal.SIGTERM)
            assert holder.wait(timeout=12) == queued.wait(timeout=12) == 0
    result = json.loads(first.read_text())
    admitted = json.loads(contender.read_text())
    if control:
        # Let the intentionally abandoned supervisor finish in the isolated
        # namespace before removing its shims and fake device state.
        wait_for(lambda: (case / "state").read_text() == "Shutdown",
                 "abandoned supervisor did not finish its simulated shutdown")
    events = [json.loads(line) for line in (case / "events.jsonl").read_text().splitlines()]
    verified = [item for item in events if item["event"] == "shutdown_seen"]
    observed = admitted["state"] == "Shutdown" and any(
        triggered < item["at"] <= admitted["at"] for item in verified)
    assert bool(observed) is not control, (name, result, events, admitted)
    assert result["exit"] == (124 if timer else 130), result
    if not control:
        witness = "worker_deadline" if timer else "worker_cancelled"
        assert witness in result["err"] and "shutdown_verified" in result["err"], result
        assert verified[-1]["at"] - triggered < 10, events
    return dict(name=name, command=command, trigger="deadline" if timer else "SIGTERM",
                triggered=triggered, first=result, device_events=events, contender=admitted,
                order_passed=bool(observed))


def main():
    host = declared("HOST", "WP4.5")
    apple = declared("APPLE-PRESENTATION", "D6.7")
    (ROOT / "tmp").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="outer-simulator-proof-", dir=ROOT / "tmp") as directory:
        tree = Path(directory)
        shims = setup(tree)
        rows = [proof_case(tree, shims, "host-cancel", host),
                proof_case(tree, shims, "presentation-cancel", apple),
                proof_case(tree, shims, "presentation-deadline", apple, timer=True),
                proof_case(tree, shims, "early-shell-control", apple, control=True)]
    output = ROOT / ".orchestrate/evidence/SESSION/WP5/simulator-lifecycle/outer.json"
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(dict(commit=commit, runner=str(SCRIPT), rows=rows), indent=2) + "\n")
    print(output)


if __name__ == "__main__":
    if len(sys.argv) > 1:
        run_check(*sys.argv[2:])
    else:
        main()
