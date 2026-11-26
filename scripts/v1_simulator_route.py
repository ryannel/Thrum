"""Build the iOS function proof and run its route on the admitted simulator."""

import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

import simulator_lifecycle as simulator

ROUTE = simulator.ROOT / "examples/v1-game-ios/ios/check-simulator.sh"


def prepare(env):
    subprocess.run(["bash", ROUTE, "--prepare"], cwd=simulator.ROOT, env=env, check=True)


def worker(turn, mode, env):
    env = turn.worker_env(env)
    if mode:
        env["THRUM_V1_ROWS"] = mode
    turn.run(["bash", ROUTE, "--device-worker"], env=env)


def presentation(mode, output, *, clock=time.monotonic, prepared=False):
    holder, _ = simulator.admission()
    env = dict(os.environ, THRUM_V1_ROUTE_OUT=str(output))
    if mode in ("reset", "g4"):
        env.update(SIMCTL_CHILD_THRUM_RECONSTRUCTION="taau", SIMCTL_CHILD_THRUM_RECIPE="portable")
    if not holder:
        prepare(env)
        command = [sys.executable, __file__, "--presentation", mode, "--output", output, "--prepared"]
        subprocess.run(simulator.lock_command(command), cwd=simulator.ROOT, check=True)
        return
    try:
        # The reset case needs fresh app and runner state. Keep OS migration
        # state across boots to avoid retriggering first-boot data migration.
        with simulator.supervise(fresh=mode != "reset", seconds=1500, clock=clock) as turn:
            if not prepared:
                turn.run(["bash", ROUTE, "--prepare"], env=env)
            if mode == "reset":
                for bundle in ("org.thrum.v1", "org.thrum.v1.uitests.xctrunner"):
                    turn.run(["xcrun", "simctl", "uninstall", turn.udid, bundle])
                turn.event("reset_apps_cleared")
            worker(turn, mode, env)
    except TimeoutError as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(124) from None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    for name in ("extent", "transition", "edr"):
        mode.add_argument(f"--{name}", dest="mode", action="store_const", const=name)
    mode.add_argument("--presentation", choices=("extent", "transition", "edr", "reset", "g4"))
    parser.add_argument("--output", type=Path)
    parser.add_argument("--prepared", action="store_true")
    args = parser.parse_args()
    if args.presentation:
        if args.output is None:
            parser.error("--presentation requires --output")
        presentation(args.presentation, args.output, prepared=args.prepared)
        return
    holder, _ = simulator.admission()
    env = dict(os.environ)
    if not holder:
        if not args.prepared:
            prepare(env)
        simulator.exec_with_admission([sys.executable, __file__, *sys.argv[1:], "--prepared"])
    env.setdefault("THRUM_V1_ROUTE_OUT", str(simulator.ROOT / "tmp" / "ios-route"))
    if "THRUM_V1_ROUTE_OUT" not in os.environ:
        root = Path(env["THRUM_V1_ROUTE_OUT"])
        root.mkdir(parents=True, exist_ok=True)
        env["THRUM_V1_ROUTE_OUT"] = str(Path(tempfile.mkdtemp(dir=root)) / "route")
    with simulator.supervise(fresh=True) as turn:
        if not args.prepared:
            turn.run(["bash", ROUTE, "--prepare"], env=env)
        worker(turn, args.mode, env)


if __name__ == "__main__":
    main()
