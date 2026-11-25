"""Run and verify the reference Crowd window and WebGPU canvas over conditioned sockets."""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

import session_sockets as sockets
import sim2d_conformance as conformance

ROOT = conformance.ROOT
SCRATCH = ROOT / "tmp/session-windowed"
TICKS = 3600
EXTENT = (640, 360)


def build(target, output):
    wasm = target == sockets.CHROME
    target_dir = ROOT / "target" / ("session-windowed-wasm" if wasm else "session-windowed")
    args = ["rustup", "run", "stable", "cargo", "build", "--locked", "--offline", "--release", "-p", "match-proof",
            "--features", "windowed", "--target-dir", str(target_dir)]
    if wasm:
        args += ["--lib", "--target", "wasm32-unknown-unknown"]
    else:
        args += ["--bin", "match-proof"]
    env = dict(os.environ, RUSTC=conformance.command(["rustup", "which", "rustc", "--toolchain", "stable"]).strip(), RUSTC_WRAPPER="")
    subprocess.run(args, cwd=ROOT, env=env, check=True)
    if wasm:
        output.mkdir()
        artifact = target_dir / "wasm32-unknown-unknown/release/match_proof.wasm"
        subprocess.run([Path.home() / ".cargo/bin/wasm-bindgen", "--target", "web", "--out-dir", output,
                        artifact], check=True)
        shutil.copy2(ROOT / "examples/match-proof/windowed_player.js", output)
        return digest(output / "match_proof_bg.wasm")
    shutil.copy2(target_dir / "release/match-proof", output)
    return digest(output)


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


class WindowMatch(sockets.Match):
    def launch_host(self):
        self.directory.mkdir()
        self.process(["caffeinate", "-d", "-i", self.programs[sockets.MAC], "window-host",
                      self.host.path, self.output / "host-control", self.ticks], self.output / "host.stderr")
        self.config = self.wait("Mac host window and ticket", lambda: self.host.events("host"), 30)[0]

    def spawn(self, peer):
        if peer.target == sockets.CHROME:
            command = ["caffeinate", "-d", "-i", "node", "scripts/session_socket_browser.mjs",
                       peer.output / "chrome", self.programs[sockets.CHROME], peer.log.path,
                       peer.output / "control", peer.output / "page", peer.ticket,
                       self.config["bodies"], "seeded", "windowed"]
        else:
            capture = peer.output / "control" if self.capture else "-"
            command = ["caffeinate", "-d", "-i", self.programs[sockets.MAC], "window-client",
                       peer.ticket, peer.log.path, capture]
        peer.process = self.process(command, peer.output / "peer.stderr")

    def tick(self):
        advances = self.host.events("advance")
        return advances[-1]["tick"] if advances else 0

    def cut(self, peer, name, duration):
        self.action("cut", name=name, duration=duration)
        control = peer.output / "relay-control"
        sockets.write_atomic(control, "cut download")
        time.sleep(duration)
        sockets.write_atomic(control, "open")
        self.action("open", name=name)


def proof(target, scenario, output, motion):
    SCRATCH.mkdir(parents=True, exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix=f"{scenario or target}-", dir=SCRATCH))
    evidence = output.absolute() if output else stage / "summary.json"
    artifacts = {sockets.MAC: stage / "match-proof"}
    hashes = {sockets.MAC: build(sockets.MAC, artifacts[sockets.MAC])}
    if target == sockets.CHROME:
        artifacts[target] = stage / "web"
        hashes[target] = build(target, artifacts[target])
    provenance = conformance.identity()
    match = WindowMatch(stage / "match", artifacts, 2_000, TICKS, listen=True)
    match.capture = motion
    subprocess.run(["caffeinate", "-u", "-t", "1"], check=True)
    print(f"Windowed {target} {scenario or 'steady'} in {stage}", flush=True)
    try:
        match.launch_host()
        peer = match.launch(target)
        match.wait("replica's independently restored lobby", lambda: peer.log.events("restore"), 45)
        match.start()
        match.wait("replica Playing", peer.playing, 25)
        run_events(match, peer, scenario, motion)
        match.finish([peer])
    finally:
        match.close()
    row = validate(match, peer, scenario)
    if motion:
        capture = peer.log.events("capture_finished")[-1]
        row["capture"] = validate_capture(peer, capture, evidence.parent / "motion")
        row["standalone"] = reference_motion(target, stage, evidence.parent / "standalone")
    evidence.parent.mkdir(parents=True, exist_ok=True)
    records = retain_records(evidence, match, peer) if output else match.output
    summary = {**provenance, **row, "target": target, "scenario": scenario or "steady", "result": "pass",
               "release_artifact_sha256": hashes, "raw_records": os.path.relpath(records, ROOT)}
    evidence.write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps({"result": "pass", "record": str(evidence)}))


def retain_records(evidence, match, peer):
    root = evidence.parent / "records"
    root.mkdir(exist_ok=True)
    shutil.copy2(match.host.path, root)
    shutil.copy2(peer.log.path, root / "client.jsonl")
    shutil.copy2(peer.output / "relay.json", root)
    if match.harness:
        shutil.copy2(match.output / "harness.json", root)
    return root


def run_events(match, peer, scenario, motion):
    is_chrome = peer.target == sockets.CHROME
    if not is_chrome and not scenario:
        return
    match.wait("240 authority ticks and settled playout", lambda: match.tick() >= 240, 15)
    if motion:
        peer.command("record")
        match.wait("first T2 image", lambda: peer.log.events("capture"), 10)
    if is_chrome:
        match.wait("10 seconds before hiding the tab", lambda: match.tick() >= 600, 15)
        peer.page("hidden")
        match.wait("document visibility became hidden", lambda: any(row["hidden"] for row in
                   peer.log.events("visibility")), 15)
        time.sleep(1.3)
        peer.page("active")
        match.wait("document visibility became visible", lambda: len(peer.log.events("visibility")) >= 3
                   and not peer.log.events("visibility")[-1]["hidden"], 15)
        match.wait("3 seconds of resumed presentation", lambda: match.tick() >= 900, 15)
    else:
        match.wait("settled playout before buffered cut", lambda: match.tick() >= 360, 15)
        match.cut(peer, "buffered", 0.018)
        match.wait("steady period before stall cut", lambda: match.tick() >= 600, 15)
        match.cut(peer, "stall", 0.23)
        match.wait("steady period before backlog cut", lambda: match.tick() >= 900, 15)
        match.cut(peer, "skip", 1.08)
        match.wait("3 seconds after skips", lambda: match.tick() >= 1170, 15)
    if motion:
        peer.command("stop")
        match.wait("T2 capture drained", lambda: peer.log.events("capture_finished"), 15)


def validate(match, peer, scenario):
    cfg = match.config
    digest_ticks = list(range(cfg["digest_interval"], TICKS + 1, cfg["digest_interval"]))
    host = match.host.events("digest")
    client = peer.log.events("digest")
    assert [row["tick"] for row in host] == digest_ticks, "host missed digest ticks"
    assert [row["tick"] for row in client] == digest_ticks, "replica missed digest ticks"
    assert [row["digest"] for row in host] == [row["digest"] for row in client], "digests differ"
    for log in (match.host, peer.log):
        renderer = log.events("renderer")[0]
        assert renderer["clock"] == "driven" and renderer["extent"] == list(EXTENT), renderer
        backend = "BrowserWebGpu" if log is peer.log and peer.target == sockets.CHROME else "Metal"
        assert renderer["backend"] == backend, renderer
        steps = [tick for row in log.events("advance") for tick in row["steps"]]
        assert steps == list(range(1, TICKS + 1)), f"missing or repeated committed ticks in {log.path}"
        assert all(len(row["steps"]) <= cfg["catch_up"] for row in log.events("advance"))
    relay = json.loads((peer.output / "relay.json").read_text())
    assert (relay["one_way_ms"], relay["jitter_ms"], relay["loss"]) == (50, 10, 0.02)
    assert all(row["forwarded"] and row["dropped"] for row in relay["directions"])
    # macOS suspends surface presents for a window fully covered by Chrome.
    # The Mac-only proof independently requires sustained drawing by both windows.
    host_frames = 1 if peer.target == sockets.CHROME else 600
    analysis = {
        "digests": len(digest_ticks), "committed_ticks_per_peer": TICKS,
        "relay": relay, "host": validate_sample(match.host, cfg, bounded_rate=False, minimum_frames=host_frames),
        "client": validate_sample(peer.log, cfg),
    }
    if scenario == "playout":
        analysis["cut_cases"] = validate_cuts(match, peer)
    if peer.target == sockets.CHROME:
        assert peer.log.events("webgpu")[0]["adapter"], "no WebGPU canvas adapter recorded"
        visibility = peer.log.events("visibility")
        assert [row["hidden"] for row in visibility] == [False, True, False], visibility
        hidden, shown = (row["wall_us"] for row in visibility[1:])
        assert 0 < shown - hidden < cfg["absence"] * cfg["tick_us"]
        assert any(row["wall_us"] >= shown for row in peer.log.events("skip") + peer.log.events("restore")), (
            "tab returned without a named skip or repair")
        assert not any(hidden + 50_000 < row["wall_us"] < shown for row in peer.log.events("advance"))
        analysis["visibility"] = visibility
    return analysis


def frame_position(row):
    return max(0, row["tick"] - 1) + row["alpha"] if row["tick"] else 0.0


def validate_sample(log, cfg, *, bounded_rate=True, minimum_frames=600):
    latest = None
    previous = None
    frames = 0
    worst_ratio = 0.0
    jump = False
    for row in log.read():
        event = row["event"]
        if event in ("skip", "restore", "finish"):
            jump = True
        elif event == "advance":
            latest = row
        elif event == "frame" and row["tick"] > 4:
            assert latest and latest["tick"] == row["tick"], (row, latest)
            frames += 1
            assert 0 <= row["alpha"] <= 1
            elapsed = max(0, row["display_us"] - latest["clock_us"])
            expected = min(row["tick"], max(row["tick"] - 1, latest["position"]) +
                           elapsed * latest["rate"] / cfg["tick_us"])
            position = frame_position(row)
            assert abs(expected - position) < 0.001, (expected, position, row, latest)
            if previous:
                elapsed = row["display_us"] - previous["display_us"]
                distance = position - frame_position(previous)
                assert elapsed > 0 and distance >= -0.001, (previous, row)
                limit = elapsed * cfg["playout_catch_up"] / 1000 / cfg["tick_us"]
                if bounded_rate and not jump:
                    worst_ratio = max(worst_ratio, distance * cfg["tick_us"] / elapsed)
                    assert distance <= limit + 0.002, (previous, row, limit, distance)
            previous, jump = row, False
    assert frames >= minimum_frames, f"insufficient actual surface submissions in {log.path}: {frames}"
    return {"frames": frames, "highest_unskipped_rate": worst_ratio,
            "stalls": log.events("stall"), "skips": log.events("skip")}


def validate_cuts(match, peer):
    events = match.harness
    result = {}
    # A recovered set can only cross after the relay reopened; allow A0 jitter,
    # a frame's observation and multiple catch-up turns for each isolated cut.
    advances = peer.log.events("advance")
    frames = peer.log.events("frame")
    for cut, opened in zip(events[::2], events[1::2], strict=True):
        name = cut["name"]
        start, end = cut["wall_us"], opened["wall_us"] + 325_000
        before = next(row for row in reversed(advances) if row["wall_us"] < start)
        window = [row for row in advances if start <= row["wall_us"] <= end]
        stalls = [row for row in peer.log.events("stall") if start <= row["wall_us"] <= end]
        skips = [row for row in peer.log.events("skip") if start <= row["wall_us"] <= end]
        bursts = [row for row in peer.log.events("burst") if opened["wall_us"] <= row["wall_us"] <= end]
        if name == "buffered":
            assert bursts and not stalls and not skips, (name, bursts, stalls, skips)
            assert all(row["rate"] == before["rate"] for row in window), (name, before, window)
        elif name == "stall":
            assert stalls and not skips, (name, stalls, skips)
            assert any(sum(abs(frame_position(frame) - stall["position"]) < 0.001
                           and start <= frame["wall_us"] < stall["wall_us"] for frame in frames) >= 2
                       for stall in stalls), "no submitted hold from the named stall"
        else:
            assert skips and all(row["first"] <= row["last"] for row in skips), (name, skips)
        result[name] = {"cut": cut, "open": opened, "bursts": bursts, "stalls": stalls, "skips": skips}
    return result


def validate_capture(peer, capture, output):
    from capture_spool import decoded_frames
    assert capture["mode"] == "realtime" and capture["omitted"] == 0, capture
    images = peer.log.events("capture")
    assert len(images) == capture["frames"] > 120
    assert all(row["extent"] == list(EXTENT) and row["draws"] == 1 and row["presented"] for row in images)
    submitted = {row["submission"]: row for row in peer.log.events("frame")}
    assert all(row["tick"] == submitted[row["submission"]]["tick"]
               and row["alpha"] == submitted[row["submission"]]["alpha"] for row in images)
    images_by_id = {row["submission"]: row for row in images}
    assert len(images_by_id) == len(images)
    assert set(range(min(images_by_id), max(images_by_id) + 1)) == images_by_id.keys(), "missing T2 submissions"
    frame_bytes = EXTENT[0] * EXTENT[1] * 4
    if peer.target == sockets.MAC:
        raw = peer.output / "frames.spool"
        frames = decoded_frames(raw, "ZstdDelta", capture["hashes"], frame_bytes, 3600)
    else:
        raw = peer.output / "frames.rgba"
        assert raw.stat().st_size == len(images) * frame_bytes

        def raw_frames():
            with raw.open("rb") as stream:
                while frame := stream.read(frame_bytes):
                    yield frame
        frames = raw_frames()
    return {**capture, **package_motion(images, frames, raw, output)}


def reference_motion(target, stage, output):
    from capture_spool import decoded_frames
    from render2d_crowd import build as build_view, replay

    build_stage = stage / "control-build"
    build_stage.mkdir()
    artifact, command = build_view(target, build_stage)
    source = stage / "standalone"
    record = replay(artifact, target, "realtime", 2000, EXTENT, 0, source)
    images = record["images"]
    assert len(images) == record["expected"] == len(record["hashes"]) > 120
    assert record["mode"] == "realtime" and all(row["extent"] == list(EXTENT)
        and row["presented"] and row["draws"] == 1 for row in images)
    raw = source / ("frames.spool" if record["encoding"] == "ZstdDelta" else "frames.rgba")
    frames = decoded_frames(raw, record["encoding"], record["hashes"], EXTENT[0] * EXTENT[1] * 4, 1600)
    artifact_file = artifact / "crowd_2d_view_bg.wasm" if target == sockets.CHROME else artifact
    return {**package_motion(images, frames, raw, output), "frames": len(images),
            "artifact_sha256": digest(artifact_file), "build_command": command}


def package_motion(images, frames, raw, output):
    from capture_v1_runtime import preview
    from render2d_crowd import encode_source_frames

    output.mkdir(parents=True, exist_ok=False)
    png = output / "png"
    png.mkdir()
    encode_source_frames(frames, images, png, EXTENT)
    start = images[0]["submit_ns"]
    records = [{"submission_id": row["submission"], "submit_ns": row["submit_ns"] - start} for row in images]
    movie = {"timebase": "realtime"}
    preview(output, movie, {"interval_end_ns": records[-1]["submit_ns"] + 16_666_667}, records)
    (output / "images.json").write_text(json.dumps(images))
    (output / "timeline.ffconcat").unlink()
    shutil.rmtree(png)
    raw.unlink()
    return {**movie, "clip": os.path.relpath(output / "preview-1x.mp4", ROOT),
            "extent": EXTENT, "seconds": records[-1]["submit_ns"] / 1e9}
