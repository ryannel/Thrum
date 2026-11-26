#!/usr/bin/env python3
"""Capture the Mac menu's fixed states and realtime interaction through T2."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from game_ui_status import prepare_store

ROOT = Path(__file__).resolve().parents[1]
EXTENTS = {
    "640x360pt": [1280, 720], "1280x720pt": [2560, 1440], "3456x2234px": [3456, 2234],
    "phone-portrait": [640, 1136], "phone-landscape": [1136, 640], "smallest": [640, 640],
}
CHOICES = ["idle", "0", "1", "2", "3", "pressed", "disabled", "disarmed"]
JOINS = ["hosting", "joining", "joining:ticket", "connecting", "joined", "refused", "closed"]


def write(path, obj):
    path.write_text(json.dumps(obj, indent=2) + "\n")


def crop(source, output, bounds):
    x, y, w, h = map(round, bounds)
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", source, "-vf",
                    f"crop={w}:{h}:{x}:{y}:exact=1", "-frames:v", "1", output], check=True)


def capture(source, mode, extent, env):
    command = [
        "python3", "scripts/capture_v1_runtime.py", "--binary", "target/release/v1-game",
        "--output", source, "--case", "sprites-menu", "--mode", mode,
        "--extent", *map(str, extent),
    ]
    subprocess.run(command, cwd=ROOT, env=env, check=True)
    manifest = json.loads((source / "manifest.json").read_text())
    assert manifest["complete"] and manifest["extent"] == extent
    return manifest, json.loads((source / "runtime.json").read_text())


def fixed(source, destination, extent, env, state):
    manifest, runtime = capture(source, "fixed", extent, dict(env, THRUM_CAPTURE_MENU_STATE=state))
    assert len(manifest["records"]) == 1
    destination.mkdir()
    frame = manifest["records"][0]["submission_id"]
    shutil.copy2(source / "png" / f"{frame:06}.png", destination / "whole.png")
    report = runtime["menu"]
    if state.startswith("choice:"):
        boxes = {node: [*origin, *size] for node, origin, size in report["boxes"]}
        assert len(boxes) == 4 and len({tuple(box[2:]) for box in boxes.values()}) == 1
        key = state.split(":")[1]
        index = int(key) if key.isdigit() else {"pressed": 1, "disabled": 3}.get(key, 0)
        crop(destination / "whole.png", destination / "card.png",
             [x * report["scale"] for x in boxes[110 + index * 10]])
    else:
        width, height = extent
        panel_width = min(width - 32, 920)
        crop(destination / "whole.png", destination / "join-panel.png",
             [(width - panel_width) / 2, (height - 488) / 2, panel_width, 488])
    for file in ("manifest.json", "runtime.json"):
        shutil.copy2(source / file, destination / file)
    shutil.rmtree(source)
    return {"state": state, "extent": extent, "submission": frame, "directory": destination.name}


def realtime(source, destination, env):
    extent = [1920, 1080]
    manifest, runtime = capture(source, "realtime", extent, env)
    destination.mkdir()
    for file in ("manifest.json", "runtime.json", "preview-1x.mp4", "contact.png"):
        shutil.copy2(source / file, destination / file)
    actions = runtime["menu_actions"]
    assert len(actions) == 12
    records = sorted(manifest["records"], key=lambda row: row["submit_ns"])
    marks = []
    for entry in actions:
        if not entry["down"] or entry["code"] == "KeyD":
            continue
        folder = destination / entry["code"]
        folder.mkdir()
        at = next(i for i, row in enumerate(records) if row["submit_ns"] >= entry["accepted_ns"])
        sequence = records[max(0, at - 2):at + 3]
        for row in sequence:
            name = f'{row["submission_id"]:06}'
            whole = folder / f"{name}.png"
            shutil.copy2(source / "png" / f"{name}.png", whole)
            crop(whole, folder / f"{name}-cards.png", [428, 211, 1064, 658])
            crop(whole, folder / f"{name}-player.png", [434, 950, 752, 122])
        marks.append({"event": entry["code"], "event_ns": entry["accepted_ns"],
                      "images": sequence})
    write(destination / "transitions.json", marks)
    shutil.rmtree(source)
    return {"extent": extent, "directory": destination.name, "frames": len(records)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--mode", choices=["fixed", "realtime", "all"], default="all")
    args = parser.parse_args()
    destination = args.output.resolve()
    destination.mkdir(parents=True)
    scratch_root = ROOT / "tmp/game-ui-wp4"
    scratch_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="stills-", dir=scratch_root) as scratch:
        scratch = Path(scratch)
        store = prepare_store(ROOT, scratch)
        env = {key: value for key, value in os.environ.items() if not key.startswith("THRUM_")}
        env.update(THRUM_SCENE="interface", THRUM_INTERFACE_TEXT="menu", THRUM_INTERFACE_SCALE="2",
                   THRUM_FONT_STORE=str(store), THRUM_MAC_FORCE_SDR="1")
        records = []
        if args.mode != "realtime":
            for size, extent in EXTENTS.items():
                states = (["choice:" + name for name in CHOICES] if size != "smallest" else [])
                if size != "3456x2234px":
                    states.extend(JOINS)
                for state in states:
                    label = f"{size}-{state.replace(':', '-')}"
                    records.append(fixed(scratch / label, destination / label, extent, env, state))
        if args.mode != "fixed":
            records.append(realtime(scratch / "realtime", destination / "realtime", env))
        write(destination / "index.json", {
            "target": "mac-metal", "commit": subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
            "binary_sha256": hashlib.file_digest((ROOT / "target/release/v1-game").open("rb"), "sha256").hexdigest(),
            "records": records,
        })


if __name__ == "__main__":
    main()
