#!/usr/bin/env python3
"""Witness saved parameter and material edits reaching the real starter without a key press.

Runs `./dev starter` in live capture mode (THRUM_CAPTURE_PNG with THRUM_CAPTURE_LIVE) on
temporary copies of the starter's parameter file and material, saves a scripted series of valid
and invalid edits, and checks each save's report line and whether its PNG changed. It exercises
the window's shared reload body offscreen, not the window's event path, and needs Metal.

Usage: python3 scripts/check_starter_live_reload.py <output folder>
"""

import json
import os
import struct
import subprocess
import sys
import threading
import time
import zlib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
STARTER = REPO / "examples" / "starter"
LIVE_SECONDS = 60
LINE_WAIT = 30.0
QUIET_WAIT = 1.5
CHANGED = 0.002
UNCHANGED = 0.0005

MOSS = """fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
    var out = surface;
    let up = smoothstep(0.55, 0.85, at.normal.y);
    out.base_colour = vec4(mix(surface.base_colour.rgb, vec3(0.10, 0.22, 0.05), up), 1.0);
    out.roughness = mix(surface.roughness, 1.0, up);
    return out;
}
"""
BROKEN = MOSS.replace("var out = surface;", "var out = surface\n")


def decode(path):
    """Decode the capture's RGB PNG (stored deflate, filter 0) into rows of bytes."""
    data = path.read_bytes()
    pos, idat = 8, b""
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        kind, body = data[pos + 4 : pos + 8], data[pos + 8 : pos + 8 + length]
        pos += 12 + length
        if kind == b"IHDR":
            width, height = struct.unpack(">II", body[:8])
        elif kind == b"IDAT":
            idat += body
    raw = zlib.decompress(idat)
    stride = width * 3
    return width, height, [raw[y * (stride + 1) + 1 : (y + 1) * (stride + 1)] for y in range(height)]


def changed_fraction(a, b):
    """Fraction of pixels where any channel differs by more than 12 codes."""
    width, height, rows_a = decode(a)
    _, _, rows_b = decode(b)
    changed = 0
    for row_a, row_b in zip(rows_a, rows_b):
        for x in range(0, width * 3, 3):
            if max(abs(row_a[x + c] - row_b[x + c]) for c in range(3)) > 12:
                changed += 1
    return changed / (width * height)


class Scene:
    def __init__(self, out, params, material):
        env = dict(os.environ)
        env.update(
            THRUM_PARAMS=str(params),
            THRUM_MATERIAL=str(material),
            THRUM_CAPTURE_PNG=str(out / "live.png"),
            THRUM_CAPTURE_LIVE=str(LIVE_SECONDS),
        )
        self.log = open(out / "starter.log", "w")
        self.process = subprocess.Popen(
            ["./dev", "starter"], cwd=REPO, env=env, stdout=subprocess.PIPE, stderr=self.log, text=True
        )
        self.lines = []
        self.ready = threading.Condition()
        threading.Thread(target=self.read, daemon=True).start()

    def read(self):
        # The cooker shares standard output; only the capture's report lines count.
        for line in self.process.stdout:
            if not line.startswith("live "):
                continue
            with self.ready:
                self.lines.append((time.monotonic(), line.rstrip("\n")))
                self.ready.notify_all()

    def next_line(self, count, wait):
        """The report line numbered `count`, or None if none arrives within `wait`."""
        deadline = time.monotonic() + wait
        with self.ready:
            while len(self.lines) <= count:
                left = deadline - time.monotonic()
                if left <= 0 or self.process.poll() is not None and len(self.lines) <= count:
                    return None
                self.ready.wait(min(left, 0.1))
            return self.lines[count]


def save_in_place(path, text):
    path.write_text(text)


def save_by_rename(path, text):
    temporary = path.with_name("." + path.name + ".saving")
    temporary.write_text(text)
    os.replace(temporary, path)


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    out = Path(sys.argv[1]).resolve()
    out.mkdir(parents=True, exist_ok=False)
    params = out / "starter.params.json"
    material = out / "material.wgsl"
    params.write_text((STARTER / "starter.params.json").read_text())
    material.write_text((STARTER / "material.wgsl").read_text())
    scene = Scene(out, params, material)
    start = scene.next_line(0, 600)
    if start is None or not start[1].startswith("live 0 start"):
        scene.process.kill()
        sys.exit(f"the starter did not start a live capture (stdout {scene.lines}); see {out / 'starter.log'}")

    def burst():
        for elevation in (31, 32, 33, 34, 15):
            save_in_place(params, json.dumps({"sun.elevation": elevation, "sun.azimuth": 150}))
            time.sleep(0.004)

    def touch():
        os.utime(params)
        save_in_place(params, params.read_text())
        save_by_rename(params, params.read_text())

    steps = [
        # name, save, expected report fragment (None: no reload), picture changes
        ("valid parameter edit", lambda: save_in_place(params, '{"sun.elevation": 40, "sun.azimuth": 150}'), "params: 1 changed, 1 unchanged, 0 refused", True),
        ("malformed parameter file", lambda: save_in_place(params, '{"sun.elevation": 12,'), "params: kept", False),
        ("refused key beside a valid one", lambda: save_in_place(params, '{"sun.elevation": 40, "sun.azimuth": 200, "sun.elevaton": 3}'), "params: 1 changed, 1 unchanged, 1 refused", True),
        ("five-write burst", burst, "params: 2 changed, 0 unchanged, 0 refused", True),
        ("touch and identical rewrites", touch, None, False),
        ("valid material tint", lambda: save_in_place(material, MOSS), "game: drawn", True),
        ("material compile error", lambda: save_by_rename(material, BROKEN), "game: kept the last good", False),
        ("parameter rename-save", lambda: save_by_rename(params, '{"sun.elevation": 50, "sun.azimuth": 120}'), "params: 2 changed, 0 unchanged, 0 refused", True),
    ]
    results, index, failures = [], 1, []
    previous = out / "live-0.png"
    for name, save, expect, changes in steps:
        saved_at = time.monotonic()
        save()
        if expect is None:
            line = scene.next_line(index, QUIET_WAIT)
            ok = line is None
            results.append({"step": name, "reloaded": line is not None, "ok": ok})
            if not ok:
                failures.append(f"{name}: unexpected reload {line[1]}")
            continue
        line = scene.next_line(index, LINE_WAIT)
        if line is None:
            failures.append(f"{name}: no report within {LINE_WAIT} s")
            break
        at, text = line
        png = out / f"live-{index}.png"
        fraction = changed_fraction(previous, png)
        ok = expect in text and (fraction > CHANGED if changes else fraction < UNCHANGED)
        # A burst must give exactly one report.
        extra = scene.next_line(index + 1, QUIET_WAIT) if name == "five-write burst" else None
        if extra is not None:
            ok = False
            failures.append(f"{name}: a second report {extra[1]}")
        results.append(
            {
                "step": name,
                "line": text,
                "changed_fraction": round(fraction, 5),
                "save_to_report_ms": round((at - saved_at) * 1000),
                "ok": ok,
            }
        )
        if not ok:
            failures.append(f"{name}: {text} (changed {fraction:.5f}, expected {'change' if changes else 'none'})")
        previous, index = png, index + 1
    scene.process.wait(timeout=LIVE_SECONDS + 60)
    scene.log.close()
    report = {"results": results, "failures": failures}
    (out / "live-reload.json").write_text(json.dumps(report, indent=2))
    for result in results:
        print(("PASS " if result["ok"] else "FAIL ") + json.dumps(result))
    if failures:
        sys.exit("FAIL: " + "; ".join(failures))
    print(f"PASS {len(results)} saves; artifacts: {out}")


if __name__ == "__main__":
    main()
