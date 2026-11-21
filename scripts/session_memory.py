"""Run and validate headless Mac session proofs with independently restored worlds."""

import hashlib
import json
import os
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

import sim2d_conformance as conformance

SCRATCH = conformance.ROOT / "tmp/session-m2"
EVIDENCE = conformance.ROOT / "tests/fixtures/session"
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
    for peer in peers:
        assert peer["ticks_held"] == record["held_by_slot"][peer["slot"]]
    for stretch, admitted_by_slot in zip(record["stretch"], record["admitted_by_slot"], strict=True):
        for peer, row in zip(peers, stretch, strict=True):
            assert (row["player"], row["slot"]) == (peer["player"], peer["slot"])
            assert row["own_frames"] == admitted_by_slot[row["slot"]]
    assert len({record["host_epoch"], *(peer["world_epoch"] for peer in peers)}) == 5
    assert all(peer["restored"][:2] == [0, record["baseline_digest"]] for peer in peers)
    assert all(peer["digest_ticks"] == len(checkpoints) for peer in peers)
    stall_us = record["stall_us"]
    if stall:
        assert 2_000_000 <= stall_us[1] - stall_us[0] < 2_009_000
    else:
        assert stall_us is None
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
    assert any(peer["player"] != peer["slot"] for peer in harsh["peers"])
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
    image_tick = record["host_image_tick"]
    assert image_tick["tick"] == 1_801 and image_tick["image_bytes"] == record["image_bytes"]
    assert image_tick["total_ns"] >= image_tick["write_ns"] + image_tick["engine_tick_ns"]
    if image_tick["total_ns"] > PERIOD_NS:
        flags.append({"name": "whole image-writing tick exceeded the authority tick period", **image_tick})
    assert {peer["repeat_depth"] for peer in record["peers"]} == {1, 2, 4, 8}
    assert len(record["stretch"][0]) == 4
    record["tripwires"] = flags
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
    assert any(peer["player"] != peer["slot"] for peer in full[1]["peers"])
    return full


def cost_rows(record):
    rows = {}

    def timing(prefix, stats):
        rows[prefix + ".mean"] = stats["total"] / stats["samples"]
        if "max" in stats:
            rows[prefix + ".max"] = stats["max"]

    for field, row in record["host_tick"].items():
        timing("host_tick." + field, row)
    timing("host_barrier_to_send_ns", record["host_barrier_to_send_ns"])
    if image_tick := record.get("host_image_tick"):
        rows["host_image_tick.total_ns"] = image_tick["total_ns"]
        rows["host_image_tick.write_ns"] = image_tick["write_ns"]

    for peer in record["peers"]:
        prefix = f"player {peer['player']} (repeat depth {peer['repeat_depth']})"
        for field, row in peer["warmed_tick"].items():
            timing(prefix + ".warmed_tick." + field, row)
        timing(prefix + ".confirmed_delay_us", peer["confirmed_delay_us"])
        if peer["stall_duration_us"]["samples"] and "held_by_slot" in record:
            timing(prefix + ".stall_duration_us", peer["stall_duration_us"])
        if "held_by_slot" in record:
            rows[prefix + ".ticks_held"] = peer["ticks_held"]
        rows[prefix + ".join_us"] = peer["join_us"]
        for field, value in peer["datagram_sets"].items():
            rows[prefix + ".datagram_sets." + field] = value
        for direction in ("to_host", "from_host"):
            for lane, row in zip(("reliable", "datagram"), peer[direction], strict=True):
                for field in ("envelope", "payload"):
                    rows[f"{prefix}.{direction}.{lane}.{field}"] = row[field]
    return rows


def compare_costs(packet, output):
    # Read retained target runs in order of preference. During a refresh, use
    # the previous contents of the output too, before writing the new record.
    candidates = [EVIDENCE / "memory-costs-review-fix.json", EVIDENCE / "memory-costs.json"]
    sources = [(path.resolve(), json.loads(path.read_text()))
               for path in dict.fromkeys([output, *candidates]) if path.is_file()]
    sources.sort(key=lambda source: source[1]["utc"], reverse=True)
    identity = ("scenario", "target", "build", "machine",
                "one_way_ms_jitter_ms_loss_ppm", "duplicate_datagrams")
    # The cost scenario always runs the reference crowd. Its wire rule identity
    # includes the toolchain and workspace inputs, which may change between
    # builds of this same workload. Keep both identities for diagnosis.
    workload = ("profile", "seed", "ticks", "bodies", "repeats")
    for record in packet["runs"]:
        current = cost_rows(record)
        for path, previous in sources:
            if any(json.dumps(previous.get(key)) != json.dumps(packet[key]) for key in identity):
                continue
            baseline = next((run for run in previous["runs"]
                             if all(run[key] == record[key] for key in workload)), None)
            if baseline is None:
                continue
            earlier = cost_rows(baseline)
            shared = current.keys() & earlier.keys()
            record["relative_regression_comparison"] = {
                "record": str(path.relative_to(conformance.ROOT) if path.is_relative_to(conformance.ROOT) else path),
                "commit": previous["commit"], "compared_rows": len(shared),
                "previous_rule_set": baseline["rule_set"], "current_rule_set": record["rule_set"],
                "previous_load_before": baseline["system_load_before"],
                "previous_load_after": baseline["system_load_after"],
            }
            for name in sorted(shared):
                if current[name] > 1.25 * earlier[name]:
                    record["tripwires"].append({
                        "name": "row more than a quarter above comparable run",
                        "row": name, "previous": earlier[name], "current": current[name],
                    })
            break
        else:
            record["relative_regression_comparison"] = "no comparable retained run"


def proof(scenario, output, build):
    provenance = conformance.identity()
    actions = {"steady": steady, "lead": lead, "costs": costs}
    SCRATCH.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=f"{scenario}-", dir=SCRATCH) as directory:
        artifact = Path(directory) / "match-proof"
        shutil.copy2(build("mac-metal"), artifact)
        artifact_hash = hashlib.sha256(artifact.read_bytes()).hexdigest()
        records = actions[scenario](artifact)
    packet = {
        **provenance, "target": "mac-metal", "build": "release", "artifact_sha256": artifact_hash,
        "one_way_ms_jitter_ms_loss_ppm": SHAPES,
        "duplicate_datagrams": True, "scenario": scenario, "runs": records, "result": "pass",
    }
    output = Path(output) if output else SCRATCH / f"memory-{scenario}.json"
    if scenario == "costs":
        compare_costs(packet, output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(packet, indent=2) + "\n")
    print(json.dumps({"result": "pass", "record": str(output)}))
