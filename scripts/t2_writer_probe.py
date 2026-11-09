#!/usr/bin/env python3
"""Measure the T2 raw-spool feasibility workload before renderer integration.

This is a disk/writer probe, not rendered-frame or runtime-motion evidence.
Uses eight reusable CPU buffers; PNG compression starts only after capture.
"""

import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import queue
import shutil
import struct
import subprocess
import threading
import time
import zlib

WIDTH, HEIGHT, RATE, COUNT = 1920, 1080, 120, 480
FRAME_BYTES = WIDTH * HEIGHT * 4
ALLOWANCE = 16 * 1024**3


def chunk(kind, payload):
    return (struct.pack(">I", len(payload)) + kind + payload
            + struct.pack(">I", zlib.crc32(kind + payload)))


def package(raw, path):
    stride = WIDTH * 4
    scanlines = b"".join(b"\0" + raw[start:start + stride]
                         for start in range(0, len(raw), stride))
    path.write_bytes(b"\x89PNG\r\n\x1a\n"
                     + chunk(b"IHDR", struct.pack(">IIBBBBB", WIDTH, HEIGHT, 8, 6, 0, 0, 0))
                     + chunk(b"IDAT", zlib.compress(scanlines, 1)) + chunk(b"IEND", b""))
    # Decode the written file, checking lengths, CRCs and exact RGBA bytes.
    png = path.read_bytes()
    offset, compressed = 8, bytearray()
    while offset < len(png):
        length = struct.unpack_from(">I", png, offset)[0]
        kind = png[offset + 4:offset + 8]
        payload = png[offset + 8:offset + 8 + length]
        crc = struct.unpack_from(">I", png, offset + 8 + length)[0]
        if zlib.crc32(kind + payload) != crc:
            raise ValueError("PNG CRC mismatch")
        if kind == b"IDAT":
            compressed.extend(payload)
        offset += length + 12
    decoded = zlib.decompress(compressed)
    rows = [decoded[start + 1:start + 1 + stride]
            for start in range(0, len(decoded), stride + 1)]
    if len(rows) != HEIGHT or any(decoded[y * (stride + 1)] != 0 for y in range(HEIGHT)):
        raise ValueError("invalid unfiltered RGBA scanlines")
    digest = hashlib.sha256(raw).hexdigest()
    if hashlib.sha256(b"".join(rows)).hexdigest() != digest:
        raise ValueError("decoded PNG hash mismatch")
    return digest


def run(output, uncached=False):
    if output.exists():
        raise ValueError("output must be a new directory")
    if shutil.disk_usage(output.parent).free < ALLOWANCE:
        raise ValueError("less than the declared 16 GiB free-space allowance")
    output.mkdir()
    console = subprocess.check_output(["/usr/sbin/ioreg", "-n", "Root", "-d1"], text=True)
    (output / "console.txt").write_text("\n".join(
        line for line in console.splitlines()
        if "IOConsoleLocked" in line or "IOConsoleUsers" in line) + "\n")
    pool, pending = queue.Queue(8), queue.Queue(8)
    # Nonzero, spatially varying RGBA data; records differ in their first pixel.
    pattern = bytes(range(256)) * (FRAME_BYTES // 256)
    for _ in range(8):
        pool.put(bytearray(pattern))
    errors, records = [], []
    spool = output / "frames.rgba"
    metrics = {}

    def writer():
        try:
            with spool.open("wb", buffering=0) as stream:
                if uncached:
                    # Darwin F_NOCACHE avoids dirty-page throttling for this
                    # single-use sequential spool; final full flush is retained.
                    fcntl.fcntl(stream.fileno(), 48, 1)
                while True:
                    job = pending.get()
                    if job is None:
                        break
                    index, buffer = job
                    begin = time.monotonic_ns()
                    view = memoryview(buffer)
                    while view:
                        written = stream.write(view)
                        if not written:
                            raise OSError("short write made no progress")
                        view = view[written:]
                    records.append({"id": index, "write_start_ns": begin,
                                    "write_end_ns": time.monotonic_ns()})
                    pool.put_nowait(buffer)
                flush = time.monotonic_ns()
                os.fsync(stream.fileno())
                # Darwin F_FULLFSYNC (51) requests the storage cache flush too.
                fcntl.fcntl(stream.fileno(), 51)
                metrics["storage_flush_ns"] = time.monotonic_ns() - flush
        except Exception as error:
            errors.append(repr(error))

    worker = threading.Thread(target=writer, daemon=True)
    worker.start()
    origin = time.monotonic_ns()
    misses, queue_peak, used_peak = [], 0, 0
    for index in range(COUNT):
        due = origin + index * 1_000_000_000 // RATE
        remaining = (due - time.monotonic_ns()) / 1e9
        if remaining > 0:
            time.sleep(remaining)
        try:
            buffer = pool.get_nowait()
        except queue.Empty:
            misses.append(index)
            continue
        struct.pack_into("<I", buffer, 0, index)
        pending.put_nowait((index, buffer))
        queue_peak = max(queue_peak, pending.qsize())
        used_peak = max(used_peak, 8 - pool.qsize())
    interval_end = time.monotonic_ns()
    pending.put(None, timeout=10)
    worker.join(timeout=30)
    metrics.update({"duration_ns": interval_end - origin,
                    "drain_including_flush_ns": time.monotonic_ns() - interval_end,
                    "writer_alive_after_deadline": worker.is_alive(),
                    "queue_peak": queue_peak, "cpu_buffers_peak": used_peak,
                    "missing_ids": misses, "errors": errors,
                    "cpu_pool_bytes": 8 * FRAME_BYTES,
                    "integrated_gpu_staging_start_bytes": 3 * FRAME_BYTES,
                    "output_allowance_bytes": ALLOWANCE,
                    "actual_spool_bytes": spool.stat().st_size,
                    "source_revision": subprocess.check_output(
                        ["git", "rev-parse", "HEAD"], text=True).strip(),
                    "probe_script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    "scope": "writer feasibility only; no GPU frames or motion proof"})
    metrics["uncached"] = uncached
    (output / "writes.json").write_text(json.dumps(records, indent=2) + "\n")
    complete = (not misses and not errors and not worker.is_alive()
                and len(records) == COUNT and spool.stat().st_size == COUNT * FRAME_BYTES)
    if complete:
        packaging = time.monotonic_ns()
        frames = output / "png"
        frames.mkdir()
        digests = []
        with spool.open("rb") as stream:
            for index in range(COUNT):
                raw = stream.read(FRAME_BYTES)
                if struct.unpack_from("<I", raw)[0] != index:
                    raise ValueError("raw record identity mismatch")
                digests.append(package(raw, frames / f"{index:04}.png"))
        (output / "hashes.json").write_text(json.dumps(digests, indent=2) + "\n")
        metrics["deferred_packaging_validation_ns"] = time.monotonic_ns() - packaging
    metrics["complete"] = complete
    metrics["retained_bytes"] = sum(path.stat().st_size for path in output.rglob("*") if path.is_file())
    (output / "result.json").write_text(json.dumps(metrics, indent=2) + "\n")
    print(json.dumps(metrics, indent=2), flush=True)
    if not complete or metrics["retained_bytes"] > ALLOWANCE:
        raise SystemExit(1)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--uncached", action="store_true")
    args = parser.parse_args()
    run(args.output, args.uncached)
