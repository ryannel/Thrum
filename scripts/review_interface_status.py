#!/usr/bin/env python3
"""Extract selected T2 status-display images and unscaled panel sequences."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from capture_spool import decoded_frames
from t2_writer_probe import package


def read(path):
    return json.loads(path.read_text())


def records(directory):
    manifest = directory / "manifest.json"
    if not manifest.exists():
        entries = [json.loads(line) for line in (directory / "status.jsonl").read_text().splitlines()]
        hashes = (directory / "hashes.txt").read_text().splitlines()
        native_timing = (directory / "images.tsv").read_text().splitlines()[1:]
        timing = {int(columns[0]): int(columns[1]) for row in native_timing
                  if (columns := row.split("\t"))}
        for entry in entries:
            entry["time_ns"] = timing[entry["submission"]]
        if len(entries) != len(hashes):
            raise ValueError("window image metadata and writer hashes disagree")
        with (directory / "frames.rgba").open("rb") as source:
            for entry, digest in zip(entries, hashes, strict=True):
                width, height = entry["extent"]
                raw = source.read(width * height * 4)
                if hashlib.sha256(raw).hexdigest() != digest:
                    raise ValueError("window spool mismatch")
                yield entry, raw
            if source.read(1):
                raise ValueError("trailing window spool data")
        return
    manifest = read(manifest)
    runtime = read(directory / "runtime.json")
    if not manifest["complete"]:
        raise ValueError("capture did not complete")
    if manifest.get("target") == "chrome-webgpu":
        with (directory / "frames.rgba").open("rb") as source:
            for entry in manifest["records"]:
                source.seek(entry["offset"])
                raw = source.read(entry["bytes"])
                if hashlib.sha256(raw).hexdigest() != entry["rgba_sha256"]:
                    raise ValueError("Chrome spool mismatch")
                yield {"extent": runtime["extent"], "scale": entry["paint_scale"],
                       "tick": entry["source_tick"], "submission": entry["submission_id"],
                       "surface_presented": entry["surface_presented"],
                       "time_ns": round(entry["submit_ms"] * 1e6)}, raw
        return
    hold = manifest["environment"].get("THRUM_CAPTURE_STATUS_TICK")
    images = runtime["images"]
    report = runtime["writer_result"]["Ok"]
    width, height = runtime["extent"]
    for entry, raw in zip(images, decoded_frames(
            directory / manifest["spool_file"], runtime["spool_encoding"],
            report["frame_sha256"], width * height * 4, manifest["record_limit"]), strict=True):
        paint = entry["interface"]
        if paint["extent_mismatch"] or paint["stamp_mismatch"] or paint["dropped_rows"]:
            raise ValueError("Metal frame had an incomplete paint list")
        yield {"extent": runtime["extent"], "scale": paint["scale"],
               "tick": int(hold) if hold is not None else entry["current_endpoint_tick"],
               "submission": entry["submission_id"],
               "surface_presented": True, "time_ns": entry["submit_ns"]}, raw


def edges(extent, scale, insets):
    width, height = extent
    left, top, right, _bottom = insets
    inner = width / scale - left - right - 16
    panel = min(304, inner)
    x = (left + 8 + (inner - panel) * 0.5) * scale
    y = (top + 8) * scale
    return int(x + 0.50000001), int(y + 0.50000001), int(x + panel * scale + 0.50000001), min(height, int(y + 116 * scale + 0.50000001))


def crop(raw, extent, bounds):
    left, top, right, bottom = bounds
    width = extent[0]
    return b"".join(
        raw[(y * width + left) * 4:(y * width + right) * 4]
        for y in range(top, bottom))


def inspect(directory, output, insets):
    output.mkdir(parents=True)
    rows = []
    crop_dir = output / "native-crops"
    crop_dir.mkdir()
    size_set = set()
    phase_samples = {}
    for entry, raw in records(directory):
        if entry["scale"] is None:
            if rows:
                raise ValueError("a resident status display disappeared")
            continue
        if not entry["surface_presented"]:
            raise ValueError("review image did not present to its surface")
        if entry.get("extent_mismatch") or entry.get("stamp_mismatch") or entry.get("dropped_rows"):
            raise ValueError("review image had an incomplete status display")
        if rows and entry["submission"] != rows[-1]["submission"] + 1:
            raise ValueError("missing submitted images in the review interval")
        bounds = edges(entry["extent"], entry["scale"], insets)
        x, y, right, bottom = bounds
        width, height = entry["extent"]
        if not 0 <= x < right <= width or not 0 <= y < bottom <= height:
            raise ValueError(f"invalid status review rectangle at {entry}")
        crop_pixels = crop(raw, entry["extent"], bounds)
        iw, ih = right - x, bottom - y
        identifier = entry["submission"]
        phase = entry["tick"] % 240
        # Compare the actual colors in the health bar, independently of the labels.
        # The color at its first pixel distinguishes the filled and empty states.
        s = entry["scale"]
        bar_y = y + 35 * s
        bar_x = x + 10 * s
        green = 0
        for at in range((bar_y * width + bar_x) * 4, (bar_y * width + right - 10 * s) * 4, 4):
            r, g, b = raw[at:at + 3]
            green += g > 180 and g > r and g > b
        track = iw - 20 * s
        expected = int(track * abs(phase - 120) / 120 + 0.5)
        if green != expected:
            raise ValueError(f"health fill {green} != {expected} at submission {identifier}")
        entry.update(bounds=bounds, box_size=[iw, ih], green_pixels=green, phase=phase,
                     rgba_sha256=hashlib.sha256(raw).hexdigest())
        package(crop_pixels, crop_dir / f"{identifier:06d}.png", iw, ih)
        key = tuple(entry["extent"]), phase
        phase_samples[key] = phase_samples.get(key, 0) + 1
        if tuple(entry["extent"]) not in size_set or (
                phase in [0, 1, 79, 80, 81, 119, 120, 121, 239] and phase_samples[key] <= 2):
            name = output / f"whole-{identifier:06d}.png"
            package(raw, name, width, height)
            entry["whole"] = name.name
            parts = {
                "bar": [bar_x, y + 33 * s, right - 10 * s, y + 39 * s],
                "timer": [right - 64 * s, y + 8 * s, right - 10 * s, y + 29 * s],
            }
            group = (iw - 32 * s) / 3
            for i, label in enumerate(("score", "gems", "wave")):
                left = x + 10 * s + int(i * (group + 6 * s) + 0.5)
                parts[label] = [left, y + 43 * s, left + round(group), y + 108 * s]
            for name, bounds in parts.items():
                l, t, r, b = bounds
                package(crop(raw, entry["extent"], bounds),
                        crop_dir / f"{identifier:06d}-{name}.png", r - l, b - t)
        size_set.add(tuple(entry["extent"]))
        rows.append(entry)
    if not rows:
        raise ValueError("capture has no submitted status display")
    (output / "frames.json").write_text(json.dumps(rows, indent=2) + "\n")
    if len(rows) > 1 and len({tuple(row["box_size"]) for row in rows}) == 1:
        concat = []
        for index, row in enumerate(rows):
            ns = row.get("time_ns", 16_666_667 * index)
            end = (rows[index + 1].get("time_ns", ns + 16_666_667)
                   if index < len(rows) - 1 else ns + 16_666_667)
            concat.extend([f"file 'native-crops/{row['submission']:06d}.png'",
                           "option framerate 1000000", f"duration {max(1, (end - ns) / 1000) / 1e6:.6f}"])
        name = output / "crops.ffconcat"
        name.write_text("ffconcat version 1.0\n" + "\n".join(concat) + "\n")
        subprocess.run(["ffmpeg", "-v", "error", "-f", "concat", "-safe", "0", "-i", name,
                        "-fps_mode", "vfr", "-c:v", "libx264rgb", "-crf", "0",
                        output / "status-1x.mp4"], check=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--insets", nargs=4, type=float, default=[0.0] * 4)
    args = parser.parse_args()
    inspect(args.capture, args.output, args.insets)
