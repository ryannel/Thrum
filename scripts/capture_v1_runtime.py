#!/usr/bin/env python3
"""Spool and validate an elapsed-host-time V1 Metal capture (no desktop API).

Run under the repository's gpu lock. The game must be built first. Each session
has a 16 GiB allowance; output must be new. Live lossless encoding runs on the writer; PNG/movie packaging is deferred.
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
from t2_load import LoadRecord
from capture_spool import decoded_frames

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


def finalize(output, manifest, runtime, verify_only=False):
    if runtime["scenario"] == "off":
        validate_runtime(runtime, 0)
        manifest.update(complete=True, packaging_pending=False, records=[])
        return
    encoding = runtime['spool_encoding']
    report = runtime['writer_result']['Ok']
    source = output / manifest['spool_file']
    if source.stat().st_size != report['encoded_bytes']:
        raise ValueError('encoded byte count differs from writer')
    records = validate_runtime(runtime, report['bytes'])
    if len(records) != len(report['frame_sha256']):
        raise ValueError('missing original frame hashes')
    frame_dir = output / 'png'
    if not verify_only:
        frame_dir.mkdir(exist_ok=True)
    begin = time.monotonic_ns()
    by_id = {row['submission_id']: row for row in runtime['images']}
    for record, raw in zip(records, decoded_frames(source, encoding, report['frame_sha256']), strict=True):
        identity = record['submission_id']
        record['rgba_sha256'] = hashlib.sha256(raw).hexdigest()
        if not verify_only:
            assert package(raw, frame_dir / f'{identity:06}.png') == record['rgba_sha256']
        record['submit_ns'] = by_id[identity]['submit_ns']
        if manifest['timebase'] == 'fixed':
            record['sample_ns'] = next(row['scenario_ns'] for row in runtime['opportunities'] if row['submission_id'] == identity)
    manifest['deferred_packaging_ns'] = time.monotonic_ns() - begin
    manifest['records'] = records
    manifest['lossless_source_archive'] = {'file': source.name, 'sha256': digest(source), 'bytes': source.stat().st_size,
        'encoding': encoding, 'decoded_frames_verified': len(records), 'timebase': 'ordinal; use submission records',
        'equality': 'independent zstd CLI + XOR reconstruction equals each original pre-encoding writer SHA-256'}
    if records and not verify_only:
        preview(output, manifest, runtime, records)
    manifest['deferred_packaging_total_ns'] = time.monotonic_ns() - begin
    manifest.update(complete=True, packaging_pending=False)


def preview(output, manifest, runtime, records):
    mode = manifest['timebase']
    pts_key = "sample_ns" if mode == "fixed" else "submit_ns"
    ordered = sorted(records, key=lambda row: row[pts_key])
    if ordered:
        lines = ["ffconcat version 1.0"]
        for index, record in enumerate(ordered):
            end = ordered[index + 1][pts_key] if index + 1 < len(ordered) else max(
                runtime["interval_end_ns"] if mode == "realtime" else 4_000_000_000,
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
    if args.finalize:
        manifest = json.loads((output / 'manifest.json').read_text())
        runtime = json.loads((output / 'runtime.json').read_text())
        try:
            finalize(output, manifest, runtime, args.verify_only)
        except Exception as error:
            manifest.update(complete=False, failure=str(error))
        save_manifest(output, manifest)
        if not manifest['complete']:
            raise SystemExit(manifest.get('failure', 'incomplete'))
        return
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
        "helper_sha256": {name: digest(ROOT / "scripts" / name) for name in ("t2_load.py", "t2_writer_probe.py", "capture_spool.py")},
        "shader_hashes": {str(p.relative_to(ROOT)): digest(p) for p in sorted((ROOT / "crates").rglob("*.wgsl"))},
        "cooked_hashes": {str(p.relative_to(store)): digest(p) for p in sorted(store.rglob("*")) if p.is_file()} if store else {},
        "scenario": args.case,
        "timebase": args.mode,
        "scenario_source_sha256": digest(ROOT / "examples/v1-game/src/window/capture.rs"),
        "parameter_sha256": digest(ROOT / "examples/v1-game/params/showcase-noon.params.json"),
        "output_allowance_bytes": ALLOWANCE,
        "cpu_pool_bytes": 96 * FRAME_BYTES,
        "gpu_staging_bytes": 6 * FRAME_BYTES,
        "pixel_byte_allowance": 1024 * 1024 * 1024,
        "encoded_queue_bound_bytes": 64 * 4 * 1024 * 1024,
        "codec_frame_scratch_bytes": 2 * FRAME_BYTES,
        "codec_workers": 2,
        "codec_window_bytes_per_worker": 4 * 1024 * 1024,
        "memory_scope": "pixel leases plus finite encoded buffers and codec scratch; allocator/driver/codec overhead measured separately by process RSS",
        "extent": [1920, 1080], "channel_order": "RGBA", "row_stride": 1920 * 4,
        "encoding": "SDR sRGB encoded bytes in Rgba8Unorm", "stage": "final engine output before identity present",
        "excluded": ["native controls", "OS composition", "physical output"],
        "gpu_timestamps_required": False, "display_swap_times": None,
        "complete": False,
        "spool_file": "frames.zst" if os.environ.get("THRUM_CAPTURE_ENCODING") != "raw" else "frames.rgba",
        "disk_free_before_bytes": shutil.disk_usage(output).free,
        "decoder_version": subprocess.check_output(["zstd", "--version"], text=True).strip(),
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
    sampler = None
    load = LoadRecord(output)
    begin = time.monotonic_ns()
    try:
        with (output / "stderr.txt").open("wb") as errors, (output / manifest["spool_file"]).open("wb", buffering=0) as spool:
            fcntl.fcntl(spool.fileno(), 48, 1)  # Darwin F_NOCACHE: measured writer recipe.
            # Inherit the uncached file description directly. A pipe added
            # another ~1 GB/s transfer with frequent kernel-buffer waits.
            process = subprocess.Popen([str(binary)], env=environment, stdout=spool, stderr=errors)
            deadline = time.monotonic() + 90
            while True:
                if args.profile and sampler is None and (output / "measurement-ready.txt").exists():
                    sampler = subprocess.Popen(["/usr/bin/sample", str(process.pid), "8", "1", "-mayDie", "-file", str(output / "cpu-sample.txt")], stdout=errors, stderr=errors)
                    manifest["cpu_profile_scope"] = "1 ms sampler requested after engine ready marker before prime; observed within 10 ms; ends on child exit; separate profiled arms"
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
        manifest["system_load"] = load.finish()
        load = None
        if sampler is not None:
            manifest["cpu_profile_returncode"] = sampler.wait(timeout=15)
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
        report = runtime['writer_result'].get('Ok', {})
        validate_runtime(runtime, report.get('bytes', 0))
        if (output / manifest['spool_file']).stat().st_size != report['encoded_bytes']:
            raise ValueError('encoded byte count differs from writer')
        manifest.update(capture_complete=True, packaging_pending=True)
        if not args.defer:
            finalize(output, manifest, runtime, args.verify_only)
    except Exception as error:
        manifest["failure"] = str(error)
        if process is not None and process.poll() is None:
            process.kill()
            process.wait()
    finally:
        if load is not None:
            manifest["system_load"] = load.finish()
        if sampler is not None and sampler.poll() is None:
            sampler.terminate()
            sampler.wait(timeout=5)
        save_manifest(output, manifest)
    print(json.dumps({key: manifest[key] for key in ("complete", "console_locked", "retained_bytes")}))
    if not manifest["complete"] and not (args.defer and manifest.get("capture_complete")):
        raise SystemExit(manifest.get("failure", "incomplete"))


def save_manifest(output, manifest):
    manifest['retained_bytes'] = sum(p.stat().st_size for p in output.rglob('*') if p.is_file())
    if manifest['retained_bytes'] > ALLOWANCE:
        manifest.update(complete=False, capture_complete=False, failure='session output allowance exceeded')
    temporary = output / 'manifest.partial.json'
    temporary.write_text(json.dumps(manifest, indent=2) + '\n')
    temporary.replace(output / 'manifest.json')


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/v1-game"))
    parser.add_argument("--store", type=Path)
    parser.add_argument("--case", choices=["pan", "running", "render-stall", "input-stall", "off", "writer-slow"], default="pan")
    parser.add_argument("--defer", action="store_true", help="record pending evidence; finalize after consecutive sessions")
    parser.add_argument("--finalize", action="store_true", help="decode/hash and finalize a previously recorded session")
    parser.add_argument("--verify-only", action="store_true", help="verify every original RGBA hash without PNG/movie duplicates")
    parser.add_argument("--profile", action="store_true", help="separate CPU sample arm; includes preparation/warmup")
    parser.add_argument("--mode", choices=["realtime", "fixed", "still"], default="realtime")
    run(parser.parse_args())
