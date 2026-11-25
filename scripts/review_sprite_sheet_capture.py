#!/usr/bin/env python3
"""Package realtime sprite-sheet captures at their measured timestamps."""

import argparse
import hashlib
import json
from pathlib import Path
import statistics
import subprocess

from capture_spool import decoded_frames
from capture_v1_runtime import preview
from sprite_sheet_fixture import COLOURS
from t2_writer_probe import package


def read_json(path):
    return json.loads(path.read_text())


def package_capture(source, output):
    manifest = read_json(source / "manifest.json")
    runtime = read_json(source / "runtime.json")
    if not manifest.get("complete") or not runtime.get("complete"):
        raise ValueError("T2 capture is incomplete")
    browser = manifest.get("target") == "chrome-webgpu"
    width, height = manifest.get("extent", runtime["extent"])
    frame_bytes = width * height * 4
    shots = runtime["images"]
    if browser:
        stamps = [round(row["submit_ms"] * 1_000_000) for row in shots]
        generations = [row["sheet_generation"] for row in shots]
        camera = [row["sprite_view_centre"] for row in shots]
        saves = [round(manifest["sprite_sheets"]["save_ms"] * 1_000_000)]
        filename = "frames.rgba"
        encoding = "Raw"
        hashes = [row["rgba_sha256"] for row in shots]
    else:
        stamps = [row["submit_ns"] for row in shots]
        generations = [row["sprites"]["sheet_generation"] for row in shots]
        camera = [row["sprites"]["view_centre"] for row in shots]
        saves = [row["accepted_ns"] for row in runtime["sheet_saves"]]
        filename = manifest["spool_file"]
        encoding = runtime["spool_encoding"]
        hashes = runtime["writer_result"]["Ok"]["frame_sha256"]
    switches = [i for i, pair in enumerate(zip(generations, generations[1:]), 1)
                if pair[0] != pair[1]]
    if [generations[0], *(generations[i] for i in switches)] != list(range(1, len(saves) + 2)):
        raise ValueError(f"unexpected sheet generations: {generations}")
    if len(shots) < 16 or len(switches) != len(saves):
        raise ValueError("missing switch samples")
    output.mkdir()
    png = output / "png"
    png.mkdir()
    records = []
    counts = []
    for n, raw in enumerate(decoded_frames(source / filename, encoding, hashes, frame_bytes, 4096)):
        row = shots[n]
        generation = generations[n]
        hits = [raw.count(bytes((*colour, 255))) for colour in COLOURS]
        if hits[generation - 1] < 64 or sum(hits) != hits[generation - 1]:
            raise ValueError(f"wrong, missing or mixed art at submission {row['submission_id']}: {hits}")
        sprites = row if browser else row["sprites"]
        if (sprites["sprite_world_rows" if browser else "world_rows"] != 500
                or sprites["pending_sheet_rows"] or sprites["missing_frame_rows"]):
            raise ValueError(f"missing rows at submission {row['submission_id']}")
        counts.append(hits)
        file = png / f"{row['submission_id']:06}.png"
        if package(raw, file, width, height) != hashes[n]:
            raise ValueError("retained frame differs from T2 source")
        records.append({"submission_id": row["submission_id"], "submit_ns": stamps[n] - stamps[0]})
    playback = {"timebase": "realtime"}
    preview(output, playback, {"interval_end_ns": stamps[-1] - stamps[0]}, records)
    comparison = [
        switch_evidence(output, shots, stamps, camera, counts, index, save)
        for index, save in zip(switches, saves, strict=True)
    ]
    receipt = {
        "target": "chrome-webgpu" if browser else "mac-metal",
        "source_revision": manifest["source_revision"],
        "mode": "realtime",
        "submitted_frames": len(records),
        "extent": [width, height],
        "timeline_origin_ns": stamps[0],
        "video_sha256": hashlib.sha256((output / "preview-1x.mp4").read_bytes()).hexdigest(),
        "preview_pts_max_error_seconds": playback["preview_pts_max_error_seconds"],
        "switches": comparison,
        "opportunities": runtime["opportunities"],
        "native_admission_skips": runtime.get("admission_skipped_opportunities"),
        "browser_spool_backpressure": runtime.get("spool_backpressure_opportunities"),
    }
    (output / "switches.json").write_text(json.dumps(receipt, indent=2) + "\n")


def switch_evidence(output, shots, stamps, camera, counts, index, save):
    if not 4 <= index < len(stamps) - 4 or stamps[index] <= save:
        raise ValueError("cannot bracket both sides of the save")
    if shots[index - 2]["tick"] >= shots[index + 2]["tick"] or camera[index - 2] == camera[index + 2]:
        raise ValueError("world or view did not move through the switch")
    first = max(1, next(i for i, stamp in enumerate(stamps) if stamp >= save))
    earlier = max(1, next(i for i, stamp in enumerate(stamps) if stamp >= save - 600_000_000))
    gaps = [right - left for left, right in zip(stamps, stamps[1:])]
    before = gaps[earlier - 1:first]
    during = gaps[first - 1:index + 2]
    identities = [row["submission_id"] for row in shots[index - 4:index + 4]]
    label = f"switch-{shots[index]['submission_id']}"
    grid(output, identities, "scale=480:270,tile=4x2", label + "-whole.png")
    grid(output, identities, "crop=480:300:720:390,tile=4x2", label + "-native.png")
    return {
        "save_ns": save, "first_new_submission": shots[index]["submission_id"],
        "switch_ns": stamps[index], "preceding_gap_ns": gaps[index - 1],
        "pre_save_median_gap_ns": statistics.median(before),
        "pre_save_max_gap_ns": max(before), "save_through_switch_max_gap_ns": max(during),
        "frames": [{"submission": shots[i]["submission_id"], "tick": shots[i]["tick"],
                    "submit_ns": stamps[i], "view_centre": camera[i], "palette_pixels": counts[i]}
                   for i in range(index - 4, index + 4)],
    }


def grid(output, identities, filters, filename):
    playlist = output / "still-grid.ffconcat"
    playlist.write_text("ffconcat version 1.0\n" + "".join(
        f"file 'png/{identity:06}.png'\n" for identity in identities))
    subprocess.run(["ffmpeg", "-v", "error", "-f", "concat", "-safe", "0", "-i",
                    playlist, "-vf", filters, "-frames:v", "1", output / filename], check=True)
    playlist.unlink()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    package_capture(args.capture.resolve(), args.output.resolve())
