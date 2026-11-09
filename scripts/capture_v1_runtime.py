#!/usr/bin/env python3
"""Spool and validate an elapsed-host-time V1 Metal capture (no desktop API).

Run under the repository's gpu lock. The game must be built first. Each session
has a 16 GiB allowance; output must be new. Compression starts after process exit.
"""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
import platform
import uuid

from t2_writer_probe import ALLOWANCE, FRAME_BYTES, package

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def read_exact(stream, length):
    result = bytearray()
    while len(result) < length:
        block = stream.read(length - len(result))
        if not block:
            raise ValueError("truncated raw record")
        result.extend(block)
    return result


def archive_sources(output, records):
    """Bounded streaming lossless RGB archive, with every decoded RGBA hash checked.

    Archive timestamps are indices only. runtime.json/manifest records own host
    times; preview-1x.mp4 is the separate elapsed-time viewing representation.
    """
    target = output / "source-lossless.mkv"
    subprocess.run(["ffmpeg", "-v", "error", "-f", "rawvideo", "-pixel_format", "rgba",
                    "-video_size", "1920x1080", "-framerate", "120", "-i", str(output / "frames.rgba"),
                    "-c:v", "libx264rgb", "-crf", "0", "-preset", "medium", "-threads", "2",
                    str(target)], check=True, timeout=120)
    process = subprocess.Popen(["ffmpeg", "-v", "error", "-i", str(target), "-f", "rawvideo",
                                "-pix_fmt", "rgba", "pipe:1"], stdout=subprocess.PIPE)
    try:
        for record in records:
            rgba = read_exact(process.stdout, FRAME_BYTES)
            if hashlib.sha256(rgba).hexdigest() != record["rgba_sha256"]:
                raise ValueError("lossless source archive decoded hash mismatch (including alpha)")
        if process.stdout.read(1):
            raise ValueError("extra source archive frames")
        if process.wait(timeout=15):
            raise ValueError("source archive decoder failed")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
    return {"file": target.name, "sha256": digest(target), "bytes": target.stat().st_size,
            "decoded_frames_verified": len(records), "timebase": "ordinal only; use retained submit_ns records"}


def validate_runtime(runtime, spool_bytes):
    """Refuse incomplete intervals and non-bijective submission/image joins."""
    if not runtime["complete"] or runtime["backend"] != "Metal":
        raise ValueError("runtime incomplete or wrong backend")
    submitted_rows = [row for row in runtime["opportunities"] if row["submitted"]]
    submitted = {row["submission_id"]: row for row in submitted_rows}
    if len(submitted) != len(submitted_rows):
        raise ValueError("duplicate submission identity")
    images = runtime["images"]
    image_ids = [row["submission_id"] for row in images]
    if runtime["scenario"] == "off":
        if images or spool_bytes:
            raise ValueError("capture-off row produced pixels")
        return []
    if (len(set(image_ids)) != len(image_ids) or set(image_ids) != set(submitted)
            or spool_bytes != len(images) * FRAME_BYTES):
        raise ValueError("incomplete runtime/record join")
    previous = -1
    for image in sorted(images, key=lambda row: row["submission_id"]):
        event = submitted[image["submission_id"]]
        if image["submit_ns"] <= previous or image["map_observed_ns"] < image["submit_ns"]:
            raise ValueError("invalid host-time order")
        if event.get("accepted_ns") is not None and image["submit_ns"] < event["accepted_ns"]:
            raise ValueError("image predates its accepted controls")
        previous = image["submit_ns"]
    return [{"submission_id": identity, "offset": index * FRAME_BYTES}
            for index, identity in enumerate(image_ids)]


def run(args):
    output = args.output.resolve()
    if output.exists():
        raise ValueError("output must be a new directory")
    output.parent.mkdir(parents=True, exist_ok=True)
    if shutil.disk_usage(output.parent).free < ALLOWANCE:
        raise ValueError("less than declared 16 GiB free space")
    output.mkdir()
    binary = args.binary.resolve()
    store = args.store.resolve() if args.store else None
    source_names = subprocess.check_output(["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard",
                                           "crates", "examples", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo"], cwd=ROOT).decode().split("\0")
    source_files = {name: digest(ROOT / name) for name in source_names
                    if name and (ROOT / name).is_file() and Path(name).suffix in (".rs", ".wgsl", ".toml", ".lock", ".json")}
    manifest = {
        "source_revision": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
        "session_id": str(uuid.uuid4()),
        "host": platform.platform(),
        "dirty_diff_sha256": hashlib.sha256(subprocess.check_output(["git", "diff", "HEAD"])).hexdigest(),
        "binary_sha256": digest(binary),
        "build_source_files": source_files,
        "build_source_digest": hashlib.sha256(json.dumps(source_files, sort_keys=True).encode()).hexdigest(),
        "script_sha256": digest(Path(__file__)),
        "shader_hashes": {str(p.relative_to(ROOT)): digest(p) for p in sorted((ROOT / "crates").rglob("*.wgsl"))},
        "cooked_hashes": {str(p.relative_to(store)): digest(p) for p in sorted(store.rglob("*")) if p.is_file()} if store else {},
        "scenario": args.case,
        "timebase": args.mode,
        "scenario_source_sha256": digest(ROOT / "examples/v1-game/src/window/capture.rs"),
        "parameter_sha256": digest(ROOT / "examples/v1-game/params/showcase-noon.params.json"),
        "output_allowance_bytes": ALLOWANCE,
        "cpu_pool_bytes": 32 * FRAME_BYTES,
        "gpu_staging_bytes": 6 * FRAME_BYTES,
        "pixel_byte_allowance": 512 * 1024 * 1024,
        "extent": [1920, 1080], "channel_order": "RGBA", "row_stride": 1920 * 4,
        "encoding": "SDR sRGB encoded bytes in Rgba8Unorm", "stage": "final engine output before identity present",
        "excluded": ["native controls", "OS composition", "physical output"],
        "gpu_timestamps_required": False, "display_swap_times": None,
        "complete": False,
    }
    probe = subprocess.check_output(["/usr/sbin/ioreg", "-n", "Root", "-d1"], text=True)
    manifest["console_locked"] = '"IOConsoleLocked" = Yes' in probe
    (output / "console.txt").write_text("\n".join(line for line in probe.splitlines()
                                                   if "IOConsoleLocked" in line or "IOConsoleUsers" in line) + "\n")
    # Read-only preflight. No API that requests a Screen Recording grant is used.
    permission = subprocess.run(["/usr/bin/swift", "-e", "import CoreGraphics; print(CGPreflightScreenCaptureAccess())"],
                                capture_output=True, text=True, timeout=30)
    manifest["screen_recording_preflight"] = permission.stdout.strip()
    manifest["screen_recording_preflight_error"] = permission.stderr if permission.returncode else None
    environment = dict(os.environ, THRUM_CAPTURE_RUNTIME=str(output), THRUM_CAPTURE_CASE=args.case, THRUM_CAPTURE_MODE=args.mode)
    if store:
        environment.update(THRUM_ASSET_STORE=str(store), THRUM_SCENE="valley")
    manifest["environment"] = {key: value for key, value in environment.items() if key.startswith("THRUM_")}
    records = []
    process = None
    begin = time.monotonic_ns()
    try:
        with (output / "stderr.txt").open("wb") as errors, (output / "frames.rgba").open("wb", buffering=0) as spool:
            fcntl.fcntl(spool.fileno(), 48, 1)  # Darwin F_NOCACHE: measured writer recipe.
            # Inherit the uncached file description directly. A pipe added
            # another ~1 GB/s transfer with frequent kernel-buffer waits.
            process = subprocess.Popen([str(binary)], env=environment, stdout=spool, stderr=errors)
            deadline = time.monotonic() + 90
            while True:
                waited, status, usage = os.wait4(process.pid, os.WNOHANG)
                if waited:
                    process.returncode = os.waitstatus_to_exitcode(status)
                    break
                if time.monotonic() >= deadline:
                    raise TimeoutError("engine capture process deadline")
                time.sleep(0.01)
            returncode = process.returncode
            # Darwin getrusage(2): ru_maxrss is bytes. This is this engine
            # child alone, before FFmpeg/PNG packaging, not all child processes.
            manifest["engine_process_usage"] = {"peak_rss_bytes": usage.ru_maxrss,
                "user_seconds": usage.ru_utime, "system_seconds": usage.ru_stime,
                "input_blocks": usage.ru_inblock, "output_blocks": usage.ru_oublock,
                "voluntary_switches": usage.ru_nvcsw, "involuntary_switches": usage.ru_nivcsw}
            flush = time.monotonic_ns()
            os.fsync(spool.fileno())
            fcntl.fcntl(spool.fileno(), 51)  # F_FULLFSYNC, not just cached acceptance.
            manifest["full_flush_ns"] = time.monotonic_ns() - flush
        manifest["capture_and_flush_ns"] = time.monotonic_ns() - begin
        if returncode:
            raise ValueError(f"game failed: exit {returncode}; see stderr.txt")
        runtime = json.loads((output / "runtime.json").read_text())
        after = subprocess.check_output(["/usr/sbin/ioreg", "-n", "Root", "-d1"], text=True)
        manifest["console_locked_after_capture"] = '"IOConsoleLocked" = Yes' in after
        manifest["locked_permission_unavailable_qualified"] = (manifest["console_locked"]
            and manifest["console_locked_after_capture"] and manifest["screen_recording_preflight"] == "false")
        manifest["resolved_recipe_digest"] = hashlib.sha256(json.dumps({
            "recipe": runtime["recipe"], "enabled_features": runtime["enabled_features"],
            "shader_hashes": manifest["shader_hashes"]}, sort_keys=True).encode()).hexdigest()
        records = validate_runtime(runtime, (output / "frames.rgba").stat().st_size)
        frame_dir = output / "png"
        frame_dir.mkdir()
        packaging = time.monotonic_ns()
        by_id = {row["submission_id"]: row for row in runtime["images"]}
        with (output / "frames.rgba").open("rb") as spool:
            for record in records:
                identity = record["submission_id"]
                record["rgba_sha256"] = package(spool.read(FRAME_BYTES), frame_dir / f"{identity:06}.png")
                record["submit_ns"] = by_id[identity]["submit_ns"]
                if args.mode == "fixed":
                    record["sample_ns"] = next(row["scenario_ns"] for row in runtime["opportunities"]
                                               if row["submission_id"] == identity)
        manifest["deferred_packaging_ns"] = time.monotonic_ns() - packaging
        if records:
            manifest["lossless_source_archive"] = archive_sources(output, records)
        pts_key = "sample_ns" if args.mode == "fixed" else "submit_ns"
        ordered = sorted(records, key=lambda row: row[pts_key])
        if ordered:
            lines = ["ffconcat version 1.0"]
            for index, record in enumerate(ordered):
                end = ordered[index + 1][pts_key] if index + 1 < len(ordered) else max(
                    runtime["interval_end_ns"] if args.mode == "realtime" else 4_000_000_000,
                    record[pts_key] + 8_333_333)
                # Quantize absolute endpoints before differencing. Flooring
                # each duration independently accumulates ~0.5 us/frame drift.
                duration_us = max(1, round(end / 1000) - round(record[pts_key] / 1000))
                lines += [f"file 'png/{record['submission_id']:06}.png'", "option framerate 1000000",
                          f"duration {duration_us / 1e6:.6f}"]
            lines += [f"file 'png/{ordered[-1]['submission_id']:06}.png'", "option framerate 1000000"]
            (output / "timeline.ffconcat").write_text("\n".join(lines) + "\n")
            subprocess.run(["ffmpeg", "-v", "error", "-f", "concat", "-safe", "0", "-i",
                            str(output / "timeline.ffconcat"), "-fps_mode", "vfr", "-c:v", "libx264",
                            "-crf", "16", "-pix_fmt", "yuv420p", str(output / "preview-1x.mp4")], check=True)
            subprocess.run(["ffmpeg", "-v", "error", "-i", str(output / "preview-1x.mp4"),
                            "-vf", "fps=4,scale=480:270,tile=4x4", "-frames:v", "1",
                            str(output / "contact.png")], check=True)
            pts = json.loads(subprocess.check_output(["ffprobe", "-v", "error", "-select_streams", "v:0",
                "-show_entries", "frame=best_effort_timestamp_time", "-of", "json", str(output / "preview-1x.mp4")]))["frames"]
            if len(pts) != len(ordered) + 1:
                raise ValueError("preview changed source-frame coverage")
            errors = [abs(float(frame["best_effort_timestamp_time"]) - record[pts_key] / 1e9)
                      for frame, record in zip(pts, ordered)]
            if max(errors) > 0.00001:
                raise ValueError("preview compressed or quantized the declared sample times")
            manifest["preview_pts_max_error_seconds"] = max(errors)
            manifest["preview_terminal_duplicate"] = True
        manifest["records"] = records
        manifest["deferred_packaging_total_ns"] = time.monotonic_ns() - packaging
        manifest["complete"] = True
    except Exception as error:
        manifest["failure"] = str(error)
        if process is not None and process.poll() is None:
            process.kill()
            process.wait()
    finally:
        manifest["retained_bytes"] = sum(p.stat().st_size for p in output.rglob("*") if p.is_file())
        if manifest["retained_bytes"] > ALLOWANCE:
            manifest.update(complete=False, failure="session output allowance exceeded")
        temporary = output / "manifest.partial.json"
        temporary.write_text(json.dumps(manifest, indent=2) + "\n")
        temporary.replace(output / "manifest.json")
    print(json.dumps({key: manifest[key] for key in ("complete", "console_locked", "retained_bytes")}))
    if not manifest["complete"]:
        raise SystemExit(manifest.get("failure", "incomplete"))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/v1-game"))
    parser.add_argument("--store", type=Path)
    parser.add_argument("--case", choices=["pan", "running", "render-stall", "input-stall", "off", "writer-slow"], default="pan")
    parser.add_argument("--mode", choices=["realtime", "fixed", "still"], default="realtime")
    run(parser.parse_args())
