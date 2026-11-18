"""Package the Chrome mirror submission ledger and lossless inspection media."""
import argparse
from collections import defaultdict
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess

from package_inspection import HEIGHT, WIDTH, digest, png, verify


def elapsed_ms(identity):
    match = re.search(r" elapsed=(\d+(?:\.\d+)?)(ns|µs|ms|s)$", identity)
    if not match:
        raise ValueError(f"missing submission clock in {identity}")
    number, unit = match.groups()
    return float(number) * {"ns": 1e-6, "µs": 1e-3, "ms": 1, "s": 1000}[unit]


def spool(source, record):
    raw = gzip.decompress((source / record["file"]).read_bytes())
    if len(raw) != WIDTH * HEIGHT * 4 or hashlib.sha256(raw).hexdigest() != record["rgba_sha256"]:
        raise ValueError(f"incomplete or changed source frame: {record['file']}")
    return raw


def movie(source, records, path):
    times = [elapsed_ms(row["identity"]) for row in records]
    pts = [round(time - times[0]) for time in times]
    if any(a >= b for a, b in zip(pts, pts[1:])):
        raise ValueError("elapsed submission times do not advance")
    # The filter uses milliseconds to keep the Matroska timeline within one
    # millisecond of each observed timestamp. Decode to check every source byte.
    expr = str(pts[-1])
    for i, timestamp in reversed(list(enumerate(pts[:-1]))):
        expr = rf"if(eq(N\,{i})\,{timestamp}\,{expr})"
    with subprocess.Popen(
        ["ffmpeg", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgba",
         "-s", f"{WIDTH}x{HEIGHT}", "-i", "-", "-vf", f"settb=1/1000,setpts={expr}",
         "-enc_time_base", "1:1000", "-fps_mode", "passthrough", "-threads", "4",
         "-c:v", "ffv1", "-level", "3", "-pix_fmt", "bgra", path],
        stdin=subprocess.PIPE,
    ) as encoder:
        for row in records:
            encoder.stdin.write(spool(source, row))
        encoder.stdin.close()
        if encoder.wait():
            raise ValueError(f"encoding failed: {path}")
    verify(path, [row["rgba_sha256"] for row in records])
    packets = subprocess.check_output([
        "ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries",
        "packet=pts", "-of", "json", path,
    ])
    if [packet["pts"] for packet in json.loads(packets)["packets"]] != pts:
        raise ValueError(f"movie moved a submission timestamp: {path}")


def package(source, destination):
    destination.mkdir()
    ledger = json.loads((source / "browser.json").read_text())
    if ledger.get("failure") or ledger.get("errors") or ledger.get("consoleErrors"):
        raise ValueError("browser run did not pass")
    arms = defaultdict(list)
    for row in ledger["records"]:
        if "presented=true" not in row["identity"]:
            raise ValueError("selected frame was not presented")
        arms[row["name"]].append(row)
    for name, records in arms.items():
        orders = [int(re.search(r" submission=(\d+)", row["identity"])[1]) for row in records]
        if orders != list(range(orders[0], orders[0] + len(orders))):
            raise ValueError(f"{name} missed an actual submission")
        if records[0]["mode"] == "fixed":
            if len(records) != 1:
                raise ValueError(f"{name} captured more than one checkpoint")
            raw = destination / f"{name}.rgba"
            raw.write_bytes(spool(source, records[0]))
            png(raw, raw.with_suffix(".png"))
            raw.unlink()
        else:
            movie(source, records, destination / f"{name}.mkv")
    (destination / "browser.json").write_text(json.dumps(ledger, indent=2) + "\n")
    files = sorted(destination.iterdir())
    (destination / "SHA256SUMS").write_text("".join(f"{digest(file)}  {file.name}\n" for file in files))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    package(args.source, args.destination)
