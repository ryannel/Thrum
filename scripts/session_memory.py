"""Run and validate headless Mac session proofs with independently restored worlds."""

import hashlib
import json
import os
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import subprocess
import time

import sim2d_conformance as conformance

SCRATCH = conformance.ROOT / "tmp/session-m2"
SEED = 29721
FULL_TICKS = 36_000
FULL_BODIES = 10_000
PERIOD_NS = 16_667_000
PEER_BYTE_TRIPWIRE = 8_000
SHAPES = {"local": (2, 0, 0), "a0": (50, 10, 20_000), "harsh": (125, 50, 50_000)}


def run(artifact, profile, ticks, bodies, *, costs=False, stall=False):
    start = time.monotonic()
    before = os.getloadavg()
    print(f"{profile}: {ticks:,} ticks, {bodies:,} bodies, costs={costs}", flush=True)
    result = subprocess.run(
        [artifact, "memory", profile, str(ticks), str(bodies), "2", str(SEED),
         str(costs).lower(), str(stall).lower()],
        capture_output=True, text=True, check=False,
    )
    if result.returncode:
        raise RuntimeError(result.stderr)
    record = json.loads(result.stdout)
    checkpoints = record.pop("checkpoints")
    assert [row[0] for row in checkpoints] == list(range(30, ticks + 1, 30))
    peers = record["peers"]
    assert len(peers) == 4 and {peer["slot"] for peer in peers} == set(range(4))
    assert len({record["host_epoch"], *(peer["world_epoch"] for peer in peers)}) == 5
    assert all(peer["restored"][:2] == [0, record["baseline_digest"]] for peer in peers)
    assert all(peer["digest_ticks"] == len(checkpoints) for peer in peers)
    assert record["stalled"] == stall
    record["digest_ticks_compared_per_peer"] = len(checkpoints)
    record["digest_trace_sha256"] = hashlib.sha256(json.dumps(checkpoints).encode()).hexdigest()
    record["elapsed_s"] = time.monotonic() - start
    record["system_load_before"] = before
    record["system_load_after"] = os.getloadavg()
    print(f"{profile}: completed in {record['elapsed_s']:.1f}s", flush=True)
    return record


def parallel_runs(artifact, profiles, ticks, bodies, **options):
    with ThreadPoolExecutor(max_workers=2) as pool:
        jobs = [pool.submit(run, artifact, profile, ticks, bodies, **options) for profile in profiles]
        return [job.result() for job in jobs]


def steady(artifact):
    records = parallel_runs(artifact, ("a0", "harsh"), FULL_TICKS, FULL_BODIES, stall=True)
    for record in records:
        for peer in record["peers"]:
            assert all(count > 0 for count in peer["carrier_sets"].values()), peer
            assert peer["reliable_sets"] > 0 and peer["late_frames"] > 0, peer
            assert peer["obsolete_set_records"] > 0, peer
    return records


def lead(artifact):
    records = parallel_runs(artifact, ("local", "a0", "harsh", "changing", "pinned"), 4_500, 2_000)
    local, a0, harsh, changing, pinned = records
    for scenario in (local, a0, harsh, changing):
        assert all(row["fraction"] >= .95 for stretch in scenario["stretch"] for row in stretch), scenario["stretch"]
    assert any(row["fraction"] < .95 for row in pinned["stretch"][0])
    assert all(row["changes"] == 0 for row in pinned["stretch"][0])
    first, harsh_stretch, last = changing["stretch"]
    for index in range(4):
        for key in ("reported_lead", "true_lead"):
            assert local["stretch"][0][index][key] < a0["stretch"][0][index][key] < harsh["stretch"][0][index][key]
            assert first[index][key] < harsh_stretch[index][key] > last[index][key]
        assert harsh_stretch[index]["changes"] > 0 and last[index]["changes"] > 0
    return records


def measurement(row):
    assert all(key in row for key in ("samples", "total", "max"))
    assert row["samples"] > 0


def warmed_tick(tick):
    for field in ("advance_ns", "engine_tick_ns", "allocations", "allocated_bytes"):
        measurement(tick[field])
    assert tick["advance_ns"]["samples"] == tick["session_ns"]["samples"]
    assert "total" in tick["session_ns"]


def tripwires(record, small):
    flags = []
    for peer, smaller in zip(record["peers"], small["peers"]):
        for direction in ("to_host", "from_host"):
            for lane in peer[direction]:
                assert all(key in lane for key in ("messages", "envelope", "payload"))
            total = sum(lane["envelope"] + lane["payload"] for lane in peer[direction])
            rate = total / (record["ticks"] * PERIOD_NS / 1e9)
            if rate > PEER_BYTE_TRIPWIRE:
                flags.append({"name": "A0 peer bandwidth above 64 kbit/s",
                              "peer": peer["player"], "direction": direction, "bytes_per_second": rate})
        assert peer["join_us"] is not None and smaller["join_us"] is not None
        if peer["join_us"] * smaller["restored"][2] > smaller["join_us"] * peer["restored"][2]:
            flags.append({"name": "join time grew faster than image size", "peer": peer["player"]})
        assert all(name in peer["datagram_sets"] for name in ("median", "p99", "max"))
        assert all(name in peer for name in ("repeat_depth", "ticks_held", "skip_advances"))
        measurement(peer["confirmed_delay_us"])
        assert all(key in peer["stall_duration_us"] for key in ("samples", "total", "max"))
        warmed_tick(peer["warmed_tick"])
    measurement(record["host_image_write_ns"])
    measurement(record["host_barrier_to_send_ns"])
    warmed_tick(record["host_tick"])
    if record["host_image_write_ns"]["max"] > PERIOD_NS:
        flags.append({"name": "baseline write exceeded the authority tick period",
                      "write_ns": record["host_image_write_ns"]["max"]})
    assert {peer["repeat_depth"] for peer in record["peers"]} == {1, 2, 4, 8}
    assert len(record["stretch"][0]) == 4
    record["tripwires"] = flags
    record["relative_regression_comparison"] = "initial measurement for this profile and workload"
    record["join_scaling_control"] = {
        "bodies": small["bodies"], "image_bytes": small["peers"][0]["restored"][2],
        "join_us": [peer["join_us"] for peer in small["peers"]],
        "baseline_transfer_us": [peer["baseline_transfer_us"] for peer in small["peers"]],
    }


def costs(artifact):
    profiles = ("a0", "harsh")
    small = parallel_runs(artifact, profiles, 120, 2_000, costs=True)
    full = parallel_runs(artifact, profiles, FULL_TICKS, FULL_BODIES, costs=True, stall=True)
    for record, control in zip(full, small):
        tripwires(record, control)
    return full


def proof(scenario, output, build):
    provenance = conformance.identity()
    artifact = build("mac-metal")
    artifact_hash = hashlib.sha256(artifact.read_bytes()).hexdigest()
    actions = {"steady": steady, "lead": lead, "costs": costs}
    records = actions[scenario](artifact)
    packet = {
        **provenance, "target": "mac-metal", "build": "release", "artifact_sha256": artifact_hash,
        "one_way_ms_jitter_ms_loss_ppm": SHAPES,
        "duplicate_datagrams": True, "scenario": scenario, "runs": records, "result": "pass",
    }
    output = Path(output) if output else SCRATCH / f"memory-{scenario}.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(packet, indent=2) + "\n")
    print(json.dumps({"result": "pass", "record": str(output)}))
