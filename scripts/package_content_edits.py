#!/usr/bin/env python3
"""Extract eight consecutive whole submissions and unscaled crops around each edit."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

from capture_v1_runtime import preview
from t2_writer_probe import package


def read(path):
    return json.loads(path.read_text())


def run(take, output, target, crops):
    output.mkdir(parents=True, exist_ok=False)
    timing = read(take / "content-edits.json")
    native = target == "mac-metal"
    source = take / "frames" if native else take
    manifest = read(source / "manifest.json")
    assert manifest["complete"], "finalize the whole T2 take first"
    width, height = manifest["extent"]
    frame_bytes = width * height * 4
    records = manifest["records"]
    if native:
        switches = {event["product"]: event["switch_submission"] for event in timing["events"]}
        raw_path = source / manifest["spool_file"]
        assert manifest["spool_file"] == "frames.rgba", "use the raw capture encoding"
    else:
        switches = {edit["product"]: edit["switched"]["submissionId"] for edit in timing["edits"]}
        raw_path = None
    selection = {}
    for product, switch in switches.items():
        index = next(i for i, row in enumerate(records) if row["submission_id"] == switch)
        selected = records[index - 4:index + 4]
        assert [row["submission_id"] for row in selected] == list(range(switch - 4, switch + 4))
        absolute_times = [row["submit_ns"] if native else round(row["submit_ms"] * 1e6) for row in selected]
        starts = [stamp - absolute_times[0] for stamp in absolute_times]
        next_record = records[index + 4]
        end_ns = (next_record["submit_ns"] if native else round(next_record["submit_ms"] * 1e6)) - absolute_times[0]
        excerpt = [{**row, "submit_ns": start} for row, start in zip(selected, starts, strict=True)]
        for row in selected:
            if raw_path:
                with raw_path.open("rb") as spool:
                    spool.seek(row["offset"])
                    raw = spool.read(frame_bytes)
            else:
                raw = gzip.decompress((source / row["file"]).read_bytes())
            assert len(raw) == frame_bytes and hashlib.sha256(raw).hexdigest() == row["rgba_sha256"]
            folder = output / product / "whole" / "png"
            folder.mkdir(parents=True, exist_ok=True)
            assert package(raw, folder / f"{row['submission_id']:06}.png", width, height) == row["rgba_sha256"]
        selection[product] = {"first_new": switch, "records": excerpt, "end_ns": end_ns}
        variants = [("whole", None), *crops]
        for label, crop in variants:
            variant = output / product / label
            if crop:
                w, h, x, y = map(int, crop.split(":"))
                assert 0 <= x < x + w <= width and 0 <= y < y + h <= height
                (variant / "png").mkdir(parents=True)
                for row in selected:
                    name = f"{row['submission_id']:06}.png"
                    subprocess.run([
                        "ffmpeg", "-v", "error", "-i", str(output / product / "whole" / "png" / name),
                        "-vf", f"crop={crop}", "-frames:v", "1", str(variant / "png" / name),
                    ], check=True)
            layout = {"timebase": "realtime"}
            preview(variant, layout, {"interval_end_ns": end_ns}, excerpt)
            selection[product][label] = layout
    (output / "selection.json").write_text(json.dumps({
        "target": target, "mode": "realtime", "commit": timing["commit"], "extent": [width, height],
        "source_manifest_sha256": hashlib.sha256((source / "manifest.json").read_bytes()).hexdigest(),
        "crops": dict(crops), "switches": selection,
    }, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("take", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--target", required=True, choices=["mac-metal", "chrome-webgpu"])
    parser.add_argument("--crop", action="append", default=[], metavar="LABEL:W:H:X:Y")
    args = parser.parse_args()
    run(args.take, args.output, args.target, [value.split(":", 1) for value in args.crop])
