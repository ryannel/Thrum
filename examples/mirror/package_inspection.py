"""Package the mirror's exact SDR submission bytes for review, with ffmpeg."""

import argparse
import csv
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import zlib

WIDTH = 2560
HEIGHT = 1440
ROW = WIDTH * 4


def digest(file):
    with file.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def chunk(tag, data):
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data))


def png(raw, path):
    data = raw.read_bytes()
    if len(data) != ROW * HEIGHT:
        raise ValueError(f"{raw} has {len(data)} bytes")
    filtered = b"".join(b"\x00" + data[start:start + ROW] for start in range(0, len(data), ROW))
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", WIDTH, HEIGHT, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(filtered, 3))
        + chunk(b"IEND", b"")
    )
    return hashlib.sha256(data).hexdigest()


def package(source, destination):
    destination.mkdir()
    shutil.copyfile(source / "adapter.txt", destination / "adapter.txt")
    manifest = {}
    for name in ["fixed", "pan", "tilt", "orbit", "sun", "mover"]:
        case = source / name
        ledger = case / "frames.csv"
        shutil.copyfile(ledger, destination / f"{name}.csv")
        with ledger.open() as stream:
            frames = list(csv.DictReader(stream))
        history = [int(row["submission"]) for row in frames]
        if history != list(range(history[0], history[0] + len(history))):
            raise ValueError(f"{name} missed a submission")
        if name == "fixed":
            for row in frames:
                raw = case / row["file"]
                row["sha256"] = png(raw, destination / raw.with_suffix(".png").name)
        else:
            movie = destination / f"{name}.mkv"
            stamp = [int(row["submitted_elapsed_ns"]) for row in frames]
            with tempfile.TemporaryDirectory(dir=destination) as temporary:
                folder = Path(temporary)
                concat = ["ffconcat version 1.0"]
                for index, row in enumerate(frames):
                    file = f"{index:04}.png"
                    row["sha256"] = png(case / row["file"], folder / file)
                    concat.extend([f"file {file}", "option framerate 1000"])
                    if index + 1 < len(stamp):
                        concat.append(f"duration {(stamp[index + 1] - stamp[index]) / 1e9:.9f}")
                timeline = folder / "motion.ffconcat"
                timeline.write_text("\n".join(concat) + "\n")
                subprocess.run([
                    "ffmpeg", "-loglevel", "error", "-safe", "0", "-i", timeline,
                    "-fps_mode", "passthrough", "-c:v", "ffv1", "-level", "3", "-threads", "4",
                    "-pix_fmt", "bgra", movie,
                ], check=True)
            # Decode the deliverable back to the exact source bytes.
            verify(movie, [row["sha256"] for row in frames])
        manifest[name] = frames
    (destination / "frames.json").write_text(json.dumps(manifest, indent=2) + "\n")
    files = sorted(destination.iterdir())
    (destination / "SHA256SUMS").write_text("".join(f"{digest(file)}  {file.name}\n" for file in files))


def verify(movie, hashes):
    with subprocess.Popen([
        "ffmpeg", "-loglevel", "error", "-i", movie, "-f", "rawvideo", "-pix_fmt", "rgba",
        "-fps_mode", "passthrough", "-",
    ], stdout=subprocess.PIPE) as decoder:
        for index, expected in enumerate(hashes):
            data = decoder.stdout.read(ROW * HEIGHT)
            if hashlib.sha256(data).hexdigest() != expected:
                decoder.terminate()
                raise ValueError(f"{movie} frame {index} changed")
        if decoder.stdout.read(1) or decoder.wait():
            raise ValueError(f"{movie} has an invalid frame count or decode")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path)
    parser.add_argument("review", type=Path)
    args = parser.parse_args()
    package(args.capture, args.review)
