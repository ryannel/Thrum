#!/usr/bin/env python3
"""Drive the ordinary Metal T2 capture against a watched sprite-sheet folder."""

import argparse
import os
from pathlib import Path
import subprocess
import time

from sprite_sheet_fixture import prepare


def capture(run):
    run.mkdir()
    source = run / "source"
    edits = run / "edits"
    store = run / "store"
    prepare(source, edits)
    with (run / "cook.log").open("w") as log:
        watcher = subprocess.Popen(
            ["target/debug/thrum-cook", "watch", source, store],
            stdout=log, stderr=subprocess.STDOUT,
        )
        try:
            deadline = time.monotonic() + 15
            while not (store / "HEAD").exists():
                if watcher.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError(f"initial sprite sheet cook unavailable; see {log.name}")
                time.sleep(0.025)
            env = dict(os.environ, THRUM_CAPTURE_ENCODING="raw", THRUM_SCENE="sprites", THRUM_SPRITE_ROWS="500",
                       THRUM_SPRITE_SHEETS_STORE=str(store),
                       THRUM_SPRITE_SHEET_WATCHED_IMAGE=str(source / "field/walk.png"),
                       THRUM_SPRITE_SHEET_EDITS=str(edits))
            subprocess.run(
                ["python3", "scripts/capture_v1_runtime.py", "--case", "sprites-sheets",
                 "--mode", "realtime", "--recipe", "portable", "--admission", "wait",
                 "--verify-only", "--output", run / "capture"],
                env=env, check=True,
            )
        finally:
            watcher.terminate()
            watcher.wait(timeout=10)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path)
    capture(parser.parse_args().run.resolve())
