#!/usr/bin/env python3
"""Run TRANSPORT target proofs. No sockets are opened by default workspace tests."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import platform
import signal
import shutil
import subprocess
import sys
import time
import tomllib

ROOT = Path(__file__).resolve().parents[1]
SCRATCH = ROOT / "tmp/transport"
RUST_BIN = Path(subprocess.check_output(["rustup", "which", "--toolchain", "stable", "rustc"], text=True).strip()).parent
ENV = {**os.environ, "PATH": str(RUST_BIN) + os.pathsep + os.environ["PATH"], "RUSTC_WRAPPER": ""}
SIM = "47494973-E92C-4194-9DE7-3C557951B608"


def run(args, **kwargs):
    return subprocess.run(list(map(str, args)), cwd=ROOT, env=kwargs.pop("env",ENV), check=True, **kwargs)


def build(target):
    SCRATCH.mkdir(parents=True,exist_ok=True)
    with (SCRATCH / "build.lock").open("w") as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        return build_locked(target)


def build_locked(target):
    triple = {"mac-metal": None, "ios-sim": "aarch64-apple-ios-sim", "chrome-webgpu": "wasm32-unknown-unknown"}[target]
    args = ["cargo", "build", "--locked", "--offline", "--release", "-p", "link-proof"]
    if triple:
        args += ["--target", triple]
    if target == "chrome-webgpu":
        args += ["--lib"]
    run(args)
    binary = ROOT / "target" / (triple or "") / "release/link-proof"
    if target == "chrome-webgpu":
        artifact = ROOT / "target/wasm32-unknown-unknown/release/link_proof.wasm"
        digest = hashlib.sha256(artifact.read_bytes()).hexdigest()
        stamp = ROOT / "target/link-proof-web/source.sha256"
        if not stamp.exists() or stamp.read_text() != digest:
            run(["wasm-bindgen", "--target", "web", "--out-dir", "target/link-proof-web", artifact])
            stamp.write_text(digest)
    return binary


def rows(path):
    result = []
    for line in path.read_text().splitlines():
        try:
            result.append(json.loads(line))
        except json.JSONDecodeError:
            if line.endswith("}"):
                raise
    return result


def start(args, path):
    output = path.open("w")
    process = subprocess.Popen(list(map(str, args)), cwd=ROOT, env=ENV, stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
    output.close()
    return process


def first_row(process, path):
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        if path.exists() and path.stat().st_size:
            return rows(path)[0]
        if process.poll() is not None:
            raise RuntimeError(f"startup exited: {path.read_text()}")
        time.sleep(0.02)
    raise RuntimeError(f"startup timeout: {path}")


def stop(process):
    if process is not None and process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


def exchange(target, output, conditioned, seconds=32, controller="bbr", case="exchange", identity="correct"):
    output.mkdir(parents=True, exist_ok=False)
    host = relay = client = None
    try:
        host = start([ROOT / "target/release/link-proof", "host", "unused", seconds + 4, controller, case], output / "host.jsonl")
        ticket = first_row(host, output / "host.jsonl")["ticket"]
        if conditioned:
            relay = start([sys.executable, "scripts/transport_relay.py", "--host", ticket.split("@")[1], "--control", output / "control", "--output", output / "relay.json"], output / "relay.jsonl")
            address = first_row(relay, output / "relay.jsonl")["address"]
            ticket = "@".join(["thrum1", address, ticket.split("@")[2]])
            if case == "startup-cut":
                (output / "control").write_text("cut")
                time.sleep(0.05)
        if identity == "hash":
            parts = ticket.split("@")
            parts[2] = f"{int(parts[2][:2],16) ^ 1:02x}" + parts[2][2:]
            ticket = "@".join(parts)
        elif identity == "version":
            ticket = ticket.replace("thrum1@", "thrum2@", 1)
        if target == "chrome-webgpu":
            client = start(["node", "scripts/transport_browser.mjs", ticket, output / "chrome", seconds, output / "browser.json", case], output / "client.log")
        elif target == "ios-sim":
            client = start(["xcrun", "simctl", "spawn", SIM, ROOT / "target/aarch64-apple-ios-sim/release/link-proof", "client", ticket, seconds, controller, case], output / "client.jsonl")
        else:
            client = start([ROOT / "target/release/link-proof", "client", ticket, seconds, controller, case], output / "client.jsonl")
        began = time.monotonic()
        if case in ["expiry","idle","kill-peer"] and identity == "correct":
            deadline = time.monotonic()+15
            while True:
                if target == "chrome-webgpu":
                    ready = (output / "browser.json.progress.json").exists()
                else:
                    ready = any("Open" in row.get("events",[]) for row in rows(output / "client.jsonl"))
                if ready:
                    break
                if time.monotonic()>deadline:
                    raise RuntimeError(f"{case} client did not open")
                time.sleep(0.01)
        if case == "expiry":
            (output / "control").write_text("cut")
            time.sleep(2.8)
            (output / "before-heal.json").write_text(json.dumps({"cut": True, "elapsed": time.monotonic() - began}))
            (output / "control").write_text("open")
        elif case == "idle" and conditioned and identity == "correct":
            time.sleep(1)
            (output / "control").write_text("cut")
        elif case == "kill-peer":
            time.sleep(2)
            if target != "chrome-webgpu":
                pid = first_row(client, output / "client.jsonl")["client_pid"]
                os.kill(pid, signal.SIGKILL)
            else:
                chrome_pid = json.loads((output / "chrome/driver.json").read_text())["chrome_pid"]
                os.killpg(chrome_pid, signal.SIGKILL)
            (output / "kill.json").write_text(json.dumps({"harness_action":"SIGKILL", "elapsed":time.monotonic()-began,"scope":"Chrome browser process group; network-service death is not claimed" if target=="chrome-webgpu" else "native client process including its I/O thread"}))
        client_status = client.wait(timeout=seconds + 30)
        if identity == "version":
            assert client_status != 0, "wrong ticket version accepted"
            return {"wrong_version_refused": True, "client_log": str(output.relative_to(ROOT))}
        if client_status and case != "kill-peer":
            raise RuntimeError(f"client failed; see {output}")
        if identity == "hash" or case == "startup-cut":
            stop(host)
        elif host.wait(timeout=20):
            raise RuntimeError(f"host failed; see {output}")
    finally:
        stop(client)
        stop(host)
        stop(relay)
        if (output / "chrome").exists():
            shutil.rmtree(output / "chrome")
    host_rows = rows(output / "host.jsonl")
    host_results = [row["result"] for row in host_rows if "result" in row]
    if case == "kill-peer":
        closed = [event for row in host_rows for event in row.get("events",[]) if event.startswith("Closed(")]
        # Chrome's surviving network service may send a normal session close after the browser
        # parent is killed. The idle-cut case separately requires the host's timeout.
        allowed = {"Closed(TimedOut)","Closed(Peer(0))","Closed(Transport)"} if target=="chrome-webgpu" else {"Closed(TimedOut)"}
        assert host_results and closed and all(event in allowed for event in closed)
        return {"host":host_rows, "harness_kill":json.loads((output / "kill.json").read_text())}
    if target == "chrome-webgpu":
        client_record = json.loads((output / "browser.json").read_text())
        client_result = client_record["result"]
    else:
        client_record = rows(output / "client.jsonl")
        client_result, = [row["result"] for row in client_record if "result" in row]
    if identity == "hash":
        assert not host_results and not client_result["open"], "wrong hash accepted"
        events = ([row["event"] for row in client_record["events"]] if target == "chrome-webgpu" else [event for row in client_record for event in row.get("events",[])])
        assert "Closed(Refused)" in events,"wrong hash did not report refusal"
        return {"wrong_hash_refused": True, "client": client_record}
    if case == "startup-cut":
        assert not host_results and not client_result["open"]
        events = ([row["event"] for row in client_record["events"]] if target == "chrome-webgpu" else [event for row in client_record for event in row.get("events",[])])
        assert "Closed(TimedOut)" in events and "Open" not in events
        return {"client":client_record, "relay":json.loads((output / "relay.json").read_text())}
    host_result, = host_results
    if case == "costs":
        costs = client_result["costs"]
        assert costs["complete"] and len(costs["transfer_ms"]) == 3
        for phase in costs["phases"]:
            for lane in ["reliable_ms","datagram_ms"]:
                assert phase[lane] and phase[lane]["count"] > 0
                assert all(key in phase[lane] for key in ["median","p95","maximum"])
        relay_record = json.loads((output / "relay.json").read_text())
        application_bytes = sum(lane["sent_bytes"] for result in [host_result,client_result] for lane in result["lanes"])
        costs["wire_bytes_per_application_byte"] = sum(row["seen_bytes"] for row in relay_record["directions"]) / application_bytes
        steady = costs["phases"][0]["datagram_ms"]
        costs["datagram_rtt_rises"] = [{"phase":phase["phase"],"quantile":quantile,"steady_ms":steady[quantile],"during_ms":phase["datagram_ms"][quantile]} for phase in costs["phases"][1:] for quantile in ["median","p95","maximum"] if phase["datagram_ms"][quantile] > steady[quantile]]
        costs["datagram_rtt_rose_during_transfer"] = bool(costs["datagram_rtt_rises"])
        history = host_result["costs"]["wire_history"]
        costs["wire_per_application_byte_grew"] = history[-1]["wire_per_application_byte"] > history[0]["wire_per_application_byte"]
        costs["host_wire_interval_ratios"] = [{"end_ms":after["elapsed_ms"],"ratio":(after["wire_bytes"]-before["wire_bytes"])/max(1,after["application_bytes"]-before["application_bytes"])} for before,after in zip(history,history[1:])]
        return {"host":host_result,"client":client_record,"relay":relay_record}
    if case == "expiry":
        for result in [host_result, client_result]:
            snapshot = result["expiry_while_stalled"]
            assert snapshot and snapshot["lanes"][1]["queued_bytes"] == 0
            assert snapshot["lanes"][1]["sent"] == snapshot["expired"] + snapshot["dropped_cap"] + snapshot["handed_on"]
            if result is host_result or target != "chrome-webgpu":
                assert snapshot["expired"] > 0, "no sender expiry during the stall"
        return {"host":host_result, "client":client_record, "relay": json.loads((output / "relay.json").read_text())}
    if case.startswith("close-") or case == "idle":
        events = [event for row in host_rows for event in row.get("events", [])]
        client_events = ([row["event"] for row in client_record["events"]] if target == "chrome-webgpu" else [event for row in client_record for event in row.get("events", [])])
        for own_events in [events, client_events]:
            assert "Open" in own_events and any(event.startswith("Closed(") for event in own_events)
        if case == "idle":
            assert "Closed(TimedOut)" in events
        elif case == "close-host":
            assert "Closed(Local(42))" in events and "Closed(Peer(42))" in client_events
        elif case == "close-load":
            assert "Closed(Local(42))" in client_events
            assert client_result["reliable_sent"] > host_result["reliable_received"], "close did not interrupt reliable sends"
        else:
            assert "Closed(Peer(42))" in events and "Closed(Local(42))" in client_events
        return {"host":host_rows,"client":client_record}
    for sender, receiver in ((client_result, host_result), (host_result, client_result)):
        assert sender["reliable_sent"] == receiver["reliable_received"], "accepted reliable messages missing"
        assert receiver["datagrams_received"] <= sender["datagrams_sent"], "duplicate datagrams"
        if case == "limits":
            assert sender["dropped_cap"] > 0 and sender["refused"][:3] == [2, 2, 1]
        elif conditioned:
            assert receiver["datagrams_received"] * 10 >= sender["datagrams_sent"] * 9, "datagram collapse"
            assert receiver["reordered"] > 0, "no datagram reordering"
            assert receiver["datagrams_received"] < sender["datagrams_sent"], "no datagram loss"
        else:
            assert receiver["datagrams_received"] > 0, "direct datagram lane delivered nothing"
        assert sender["lanes"][0]["sent"] == sender["reliable_sent"]
        assert sender["lanes"][1]["received"] == sender["datagrams_received"]
    record = {"host": host_result, "client": client_record}
    if conditioned:
        record["relay"] = json.loads((output / "relay.json").read_text())
        for direction in record["relay"]["directions"]:
            assert direction["forwarded"] > 0 and direction["dropped"] > 0
        assert host_result["rtt_us"] >= 70_000, "conditioned RTT absent"
    return record


def reconnect(target, output):
    output.mkdir()
    host = start([ROOT / "target/release/link-proof", "host", "unused", 18, "bbr", "close-client"], output / "host.jsonl")
    clients = []
    try:
        ticket = first_row(host, output / "host.jsonl")["ticket"]
        for index in range(2):
            if target == "chrome-webgpu":
                process = start(["node","scripts/transport_browser.mjs",ticket,output / f"chrome{index}",3,output / f"client{index}.json","close-client"],output / f"client{index}.log")
            else:
                binary = ROOT / "target" / ("aarch64-apple-ios-sim" if target == "ios-sim" else "") / "release/link-proof"
                command = (["xcrun","simctl","spawn",SIM] if target == "ios-sim" else []) + [binary,"client",ticket,3,"bbr","close-client"]
                process = start(command,output / f"client{index}.jsonl")
            clients.append(process)
            assert process.wait(timeout=25) == 0
        assert host.wait(timeout=35) == 0
        result = rows(output / "host.jsonl")
        ids = [row["id"] for row in result if "Open" in row.get("events",[])]
        assert len(ids) == 2 and ids[0] != ids[1], "reconnect reused a link"
        assert sum("Closed(Peer(42))" in row.get("events",[]) for row in result) == 2
        records = [json.loads((output / f"client{i}.json").read_text()) if target == "chrome-webgpu" else rows(output / f"client{i}.jsonl") for i in range(2)]
        return {"host":result, "clients":records, "old_id":ids[0], "new_id":ids[1]}
    finally:
        for process in clients:
            stop(process)
        stop(host)
        for profile in output.glob("chrome*"):
            if profile.is_dir():
                shutil.rmtree(profile)


def host_drop(output):
    """Drop a native host that has accepted, retained and loaded links; the program checks each handle."""
    output.mkdir()
    result = subprocess.run([str(ROOT / "target/release/link-proof"), "host-drop"], cwd=ROOT, env=ENV, capture_output=True, text=True, timeout=60)
    (output / "host-drop.jsonl").write_text(result.stdout + result.stderr)
    record = json.loads(result.stdout.strip().splitlines()[-1])
    assert result.returncode == 0 and record["pass"] is True, result.stdout + result.stderr
    return record


def in_process(command, output):
    """The hostile and overload runs: one program holds the host, three healthy links and the offenders."""
    output.mkdir()
    result = subprocess.run([str(ROOT / "target/release/link-proof"), command], cwd=ROOT, env=ENV, capture_output=True, text=True, timeout=600)
    (output / f"{command}.json").write_text(result.stdout)
    (output / f"{command}.stderr").write_text(result.stderr)
    record = json.loads(result.stdout.strip().splitlines()[-1])
    assert result.returncode == 0 and record["pass"] is True, f"{command} failed: {record.get('failures')}"
    return record


IDLE_MS = 4_000


def link_rows(path):
    return rows(path) if path.exists() else []


def native_suspend(target, output):
    """Freeze a native client past the idle timeout with SIGSTOP, resume it and watch both ends."""
    output.mkdir()
    host = client = None
    try:
        host = start([ROOT / "target/release/link-proof", "host", "unused", 60, "bbr", "suspend"], output / "host.jsonl")
        host_start = first_row(host, output / "host.jsonl")
        ticket = host_start["ticket"]
        binary = ROOT / "target" / ("aarch64-apple-ios-sim" if target == "ios-sim" else "") / "release/link-proof"
        command = (["xcrun", "simctl", "spawn", SIM] if target == "ios-sim" else []) + [binary, "client", ticket, 16, "bbr", "suspend"]
        client = start(command, output / "client.jsonl")
        pid = first_row(client, output / "client.jsonl")["client_pid"]
        deadline = time.monotonic() + 15
        while not any("Open" in row.get("events", []) for row in link_rows(output / "client.jsonl")):
            assert time.monotonic() < deadline, "client did not open"
            time.sleep(0.01)
        time.sleep(3)
        freeze_ms = time.time() * 1000
        os.kill(pid, signal.SIGSTOP)
        time.sleep((IDLE_MS + 3_000) / 1000)
        resume_ms = time.time() * 1000
        os.kill(pid, signal.SIGCONT)
        assert client.wait(timeout=60) == 0, "client failed"
        time.sleep(0.5)
    finally:
        stop(client)
        stop(host)
    host_rows = rows(output / "host.jsonl")
    client_rows = rows(output / "client.jsonl")
    epoch = host_start["epoch_unix_ms"]
    opened = [row["id"] for row in host_rows if "Open" in row.get("events", [])]
    assert len(opened) >= 2 and opened[0] != opened[1], f"host saw no second link: {opened}"
    first_close = next(row for row in host_rows if row.get("id") == opened[0] and any(e.startswith("Closed(") for e in row.get("events", [])))
    host_close_ms = epoch + first_close["elapsed_ms"]
    host_close_after_freeze = host_close_ms - freeze_ms
    assert "Closed(TimedOut)" in first_close["events"], first_close
    assert 0 < host_close_after_freeze <= IDLE_MS + 1_000, f"host reported the timeout {host_close_after_freeze:.0f} ms after the freeze"
    resumed = next(row for row in client_rows if "resumed_after_gap_ms" in row)
    assert resumed["resumed_after_gap_ms"] >= IDLE_MS, resumed
    old_at_resume = resumed["links_at_resume"][0]
    old_closed = next(row["closed_result"] for row in client_rows if "closed_result" in row and row["closed_result"]["id"] == old_at_resume["id"])
    old_events = [e for row in client_rows if row.get("id") == old_at_resume["id"] for e in row.get("events", [])]
    assert any(e.startswith("Closed(") for e in old_events), "client never reported the old link closed"
    for key in ["reliable_received", "datagrams_received"]:
        assert old_closed[key] == old_at_resume[key], f"old link delivered {key} after the resume"
    new_ids = [row["id"] for row in client_rows if "Open" in row.get("events", []) and row["id"] != old_at_resume["id"]]
    assert new_ids, "client opened no new link"
    new_result = next(row["result"] for row in client_rows if "result" in row and row["result"]["id"] == new_ids[0])
    assert new_result["reliable_received"] > 0 and new_result["reliable_sent"] > 0, "the new link carried nothing"
    return {
        "idle_timeout_ms": IDLE_MS, "freeze_unix_ms": freeze_ms, "resume_unix_ms": resume_ms,
        "frozen_ms": resume_ms - freeze_ms, "host_link_closed_ms_after_freeze": host_close_after_freeze,
        "host_old_link": opened[0], "host_new_link": opened[1], "host_close_event": first_close,
        "client_gap": {k: resumed[k] for k in ["resumed_after_gap_ms", "unix_ms"]},
        "client_old_link_at_resume": old_at_resume, "client_old_link_closed": old_closed, "client_old_link_events": old_events,
        "client_new_link": new_result,
        "host_events": [row for row in host_rows if "events" in row],
    }


def chrome_suspend(output):
    """Reload a page, close a tab, then freeze the renderer, each without closing the link first."""
    output.mkdir()
    host = driver = None
    try:
        host = start([ROOT / "target/release/link-proof", "host", "unused", 90, "bbr", "suspend"], output / "host.jsonl")
        host_start = first_row(host, output / "host.jsonl")
        driver = start(["node", "scripts/transport_browser.mjs", host_start["ticket"], output / "chrome", 60, output / "browser.json", "suspend"], output / "driver.log")
        assert driver.wait(timeout=150) == 0, (output / "driver.log").read_text()[-3000:]
        time.sleep(0.5)
    finally:
        stop(driver)
        stop(host)
        if (output / "chrome").exists():
            shutil.rmtree(output / "chrome")
    browser = json.loads((output / "browser.json").read_text())
    steps = {step["step"]: step for step in browser["steps"]}
    host_rows = rows(output / "host.jsonl")
    epoch = host_start["epoch_unix_ms"]
    opened = [(row["id"], epoch + row["elapsed_ms"]) for row in host_rows if "Open" in row.get("events", [])]
    def closed_at(link):
        row = next((row for row in host_rows if row.get("id") == link and any(e.startswith("Closed(") for e in row.get("events", []))), None)
        return row and {"events": row["events"], "unix_ms": epoch + row["elapsed_ms"]}
    assert len(opened) >= 3, f"host saw {len(opened)} links, expected one per page load"
    result = {"idle_timeout_ms": IDLE_MS, "chrome": browser["version"], "steps": browser["steps"], "pages": browser["pages"]}
    for index, step in enumerate(["reload", "close-tab"]):
        link, _ = opened[index]
        closed = closed_at(link)
        assert closed, f"host never reported the link closed after {step}"
        after = closed["unix_ms"] - steps[step]["unix_ms"]
        assert 0 <= after <= IDLE_MS + 1_000, f"{step}: host reported the close {after:.0f} ms later"
        next_link, next_open = opened[index + 1]
        assert next_link != link and next_open > steps[step]["unix_ms"], f"{step}: no new link"
        result[step] = {"old_host_link": link, "host_close": closed, "host_close_ms_after_step": after, "new_host_link": next_link, "new_link_open_ms_after_step": next_open - steps[step]["unix_ms"]}
    frozen_link, _ = opened[2]
    freeze, resume = steps["freeze"]["unix_ms"], steps["resume"]["unix_ms"]
    progress = [{"host_ms": epoch + row["elapsed_ms"], "link": item} for row in host_rows if "progress" in row for item in row["progress"] if item["id"] == frozen_link]
    result["freeze"] = {
        "host_link": frozen_link, "frozen_ms": resume - freeze, "renderer_pids": steps["freeze"].get("pids"),
        "host_close_during_or_after": closed_at(frozen_link),
        "host_progress_from_before_freeze_to_after_resume": [p for p in progress if freeze - 2_000 <= p["host_ms"] <= resume + 3_000],
    }
    return result


def session(output):
    """Ten wall-clock minutes, with live sends interrupted by each fault."""
    for target in ["mac-metal","chrome-webgpu","ios-sim"]:
        build(target)
    host = start([ROOT / "target/release/link-proof","host","unused",680,"bbr","session",output / "host-control"], output / "host.jsonl")
    processes = [host]
    clients = []
    relays = []
    events = []
    pairs = []
    def host_ids():
        return [row["id"] for row in rows(output / "host.jsonl") if "Open" in row.get("events",[])]
    def await_open(previous):
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            ids = host_ids()
            if len(ids) > previous:
                return ids[-1]
            if host.poll() is not None:
                raise RuntimeError("session host exited")
            time.sleep(0.02)
        raise RuntimeError("session link failed to open")
    def launch(index, ticket, serial=0):
        target = ["mac-metal","mac-metal","chrome-webgpu","ios-sim"][index]
        stem = f"client-{index}-{serial}"
        control = output / f"control-{index}"
        control.write_text("run")
        if target == "chrome-webgpu":
            process = start(["node","scripts/transport_browser.mjs",ticket,output / stem,680,output / f"{stem}.json","session",control],output / f"{stem}.log")
        else:
            binary = ROOT / "target" / ("aarch64-apple-ios-sim" if target == "ios-sim" else "") / "release/link-proof"
            args = (["xcrun","simctl","spawn",SIM] if target == "ios-sim" else []) + [binary,"client",ticket,680,"bbr","session",control]
            process = start(args,output / f"{stem}.jsonl")
        processes.append(process)
        return {"process":process,"target":target,"stem":stem,"control":control,"ticket":ticket,"local_id":1}
    try:
        host_start = first_row(host,output / "host.jsonl")
        ticket = host_start["ticket"]
        for index in range(4):
            relay = start([sys.executable,"scripts/transport_relay.py","--host",ticket.split("@")[1],"--seed",29721+index,"--control",output / f"relay-control-{index}","--output",output / f"relay-{index}.json"],output / f"relay-{index}.jsonl")
            processes.append(relay)
            relays.append(relay)
            address = first_row(relay,output / f"relay-{index}.jsonl")["address"]
            client_ticket = "@".join(["thrum1",address,ticket.split("@")[2]])
            count = len(host_ids())
            client = launch(index,client_ticket)
            client["host_id"] = await_open(count)
            clients.append(client)
            pairs.append({"client":index,"stem":client["stem"],"local_id":1,"host_id":client["host_id"]})
            deadline = time.monotonic()+15
            while True:
                ready = (output / f"{client['stem']}.json.progress.json").exists() if client["target"]=="chrome-webgpu" else any("Open" in row.get("events",[]) for row in rows(output / f"{client['stem']}.jsonl"))
                if ready:
                    break
                if time.monotonic()>deadline:
                    raise RuntimeError("session client did not observe its open")
                time.sleep(0.01)
        began = time.monotonic()
        host_session_start_ms = time.time()*1000-host_start["epoch_unix_ms"]
        schedule = [(60,"reconnect",0),(120,"reconnect",2),(180,"reconnect",3),(240,"kill",1),(250,"replace",1),(300,"cut",0),(310,"heal-reconnect",0),(600,"drain",None),(610,"finish",None)]
        for at, action, index in schedule:
            while time.monotonic()-began < at:
                if host.poll() is not None:
                    raise RuntimeError("session host exited during load")
                for client in clients:
                    if not client.get("killed") and client["process"].poll() is not None:
                        raise RuntimeError(f"session client exited during load: {client['stem']}")
                time.sleep(0.5)
            event = {"harness_action":action,"client":index,"elapsed_seconds":time.monotonic()-began}
            if action in ["reconnect","heal-reconnect"]:
                client = clients[index]
                event["old_host_id"] = client["host_id"]
                previous = len(host_ids())
                if action == "heal-reconnect":
                    (output / f"relay-control-{index}").write_text("open")
                client["control"].write_text("close")
                deadline = time.monotonic()+8
                while not any(row.get("id")==event["old_host_id"] and any(e.startswith("Closed(") for e in row.get("events",[])) for row in rows(output / "host.jsonl")):
                    if time.monotonic()>deadline:
                        raise RuntimeError("peer did not observe the close before reconnect")
                    time.sleep(0.02)
                client["control"].write_text("reconnect")
                client["host_id"] = await_open(previous)
                client["local_id"] += 1
                event["new_host_id"] = client["host_id"]
                assert event["new_host_id"] != event["old_host_id"]
                pairs.append({"client":index,"stem":client["stem"],"local_id":client["local_id"],"host_id":client["host_id"]})
                client["control"].write_text("run")
            elif action == "kill":
                client = clients[index]
                pid = first_row(client["process"],output / f"{client['stem']}.jsonl")["client_pid"]
                event["host_id"] = client["host_id"]
                os.kill(pid,signal.SIGKILL)
                client["killed"] = True
            elif action == "replace":
                previous = len(host_ids())
                clients[index] = launch(index,clients[index]["ticket"],1)
                clients[index]["host_id"] = await_open(previous)
                pairs.append({"client":index,"stem":clients[index]["stem"],"local_id":1,"host_id":clients[index]["host_id"]})
            elif action == "cut":
                event["host_id"] = clients[index]["host_id"]
                (output / f"relay-control-{index}").write_text("cut")
            else:
                (output / "host-control").write_text(action)
                for client in clients:
                    client["control"].write_text(action)
            events.append(event)
            (output / "events.json").write_text(json.dumps(events,indent=2)+"\n")
        for client in clients:
            assert client["process"].wait(timeout=15) == 0
        assert host.wait(timeout=15) == 0
    finally:
        for process in reversed(processes):
            stop(process)
        for profile in output.glob("client-*"):
            if profile.is_dir():
                shutil.rmtree(profile)
    host_rows = rows(output / "host.jsonl")
    final = {row["result"]["id"]:row["result"] for row in host_rows if "result" in row}
    ended = {row["closed_result"]["id"]:row["closed_result"] for row in host_rows if "closed_result" in row}
    killed_host = next(event["host_id"] for event in events if event["harness_action"] == "kill")
    comparisons = []
    for pair in pairs:
        host_result = ended.get(pair["host_id"],final.get(pair["host_id"]))
        assert host_result, pair
        if pair["host_id"] == killed_host:
            comparisons.append({**pair,"killed_client_receipt":"absent","host":host_result})
            continue
        client_file = output / f"{pair['stem']}.json"
        if client_file.exists():
            client_record = json.loads(client_file.read_text())
            client_results = client_record["closed_results"] + [client_record["result"]]
        else:
            client_rows = rows(output / f"{pair['stem']}.jsonl")
            client_results = [row[key] for row in client_rows for key in ["result","closed_result"] if key in row]
        client_result = next((result for result in reversed(client_results) if result["id"] == pair["local_id"]),None)
        assert client_result, pair
        for sender, receiver in [(host_result,client_result),(client_result,host_result)]:
            assert sender["identity"] == receiver["peer_identity"], "cross-link payload identity"
            if pair["host_id"] in ended:
                assert receiver["reliable_received"] <= sender["reliable_sent"]
            else:
                assert receiver["reliable_received"] == sender["reliable_sent"]
        comparisons.append({**pair,"host":host_result,"client":client_result})
    cut_host = next(event["host_id"] for event in events if event["harness_action"] == "cut")
    cut_comparison = next(row for row in comparisons if row["host_id"] == cut_host)
    assert cut_comparison["host"]["reliable_sent"] > cut_comparison["client"]["reliable_received"], "cut drained before interruption"
    kill_time = host_session_start_ms+next(event["elapsed_seconds"]*1000 for event in events if event["harness_action"] == "kill")
    after_kill = [item for row in host_rows if row.get("elapsed_ms",0) > kill_time for item in row.get("progress",[]) if item["id"] == killed_host and item["open"]]
    assert len(after_kill) >= 2 and after_kill[-1]["reliable_sent"] > after_kill[0]["reliable_sent"], "host accepted nothing after kill"
    return {"duration_seconds":610,"load_seconds_after_all_clients_open":600,"host_session_start_ms":host_session_start_ms,"harness_events":events,"links":comparisons,"host_events":[row for row in host_rows if "events" in row],"accepted_after_kill":after_kill,"relays":[json.loads((output / f"relay-{i}.json").read_text()) for i in range(4)]}


def closure(output):
    baseline = json.loads((ROOT / "tests/fixtures/transport/lock-before.json").read_text())
    locked = tomllib.loads((ROOT / "Cargo.lock").read_text())["package"]
    present = {(p["name"], p["version"]) for p in locked}
    names = {name for name, _ in present}
    # A version moved when a package from before this module is still locked but not at that version. A package
    # that another change has since removed from the workspace or the lockfile is not a moved version.
    moved = [pair for pair in baseline if tuple(pair) not in present and pair[0] in names]
    removed = [pair for pair in baseline if pair[0] not in names]
    assert not moved, f"an existing locked version moved: {moved}"
    core = run(["cargo", "tree", "--locked", "--offline", "-p", "thrum-transport", "--no-default-features", "--prefix", "none"], capture_output=True, text=True).stdout
    assert len(core.strip().splitlines()) == 1, core
    shipping = {}
    for package in ["v1-game", "v1-game-ios", "thrum-starter", "thrum-stones", "thrum-lighthouse"]:
        tree = run(["cargo", "tree", "--locked", "--offline", "-p", package, "--target", "all"], capture_output=True, text=True).stdout
        assert "thrum-transport" not in tree
        shipping[package] = "transport absent"
    metadata = json.loads(run(["cargo", "metadata", "--locked", "--offline", "--format-version", "1", "--features", "thrum-transport/native,thrum-transport/browser"], capture_output=True, text=True).stdout)
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    packages = {package["id"]: package for package in metadata["packages"]}
    transport, = [package["id"] for package in metadata["packages"] if package["name"] == "thrum-transport"]
    pending = [transport]
    reached = set()
    while pending:
        item = pending.pop()
        if item in reached:
            continue
        reached.add(item)
        pending.extend(nodes[item]["dependencies"])
    # The crates this module added to the lockfile: in the adapters' closure and not locked before it began.
    before = {tuple(pair) for pair in baseline}
    added = sorted({(packages[item]["name"], packages[item]["version"]) for item in reached - {transport}} - before)
    assert added, "the adapter closure added nothing, so the check below would prove nothing"
    members = sorted(packages[item]["name"] for item in metadata["workspace_members"])
    closures = {}
    for package in members:
        tree = run(["cargo", "tree", "--locked", "--offline", "-p", package, "--target", "all", "-e", "normal,build", "--prefix", "none"], capture_output=True, text=True).stdout
        present = {tuple(line.split()[:2]) for line in tree.splitlines() if line.strip()}
        present = {(name, version.lstrip("v")) for name, version in present}
        found = sorted(f"{name} {version}" for name, version in present if (name, version) in set(added))
        if package == "thrum-transport":
            # The crate that holds the adapters, behind its features: with none, it reaches nothing added.
            assert not found, f"thrum-transport without features reaches {found[:5]}"
            closures[package] = {"asks_for_adapters": False, "adapters_behind_features": ["native", "browser"], "added_crates_in_closure": 0}
        elif package == "link-proof":
            closures[package] = {"asks_for_adapters": True, "added_crates_in_closure": len(found)}
        else:
            assert not found, f"{package} reaches crates this module added: {found[:5]}"
            assert not any(name == "thrum-transport" for name, _ in present), f"{package} reaches thrum-transport"
            closures[package] = {"asks_for_adapters": False, "added_crates_in_closure": 0, "thrum_transport_in_closure": False}
    # Only a package that asks for the socket adapters may reach them: link-proof names the features.
    assert closures["link-proof"]["added_crates_in_closure"] > 0
    build_core = run(["cargo", "build", "--locked", "--offline", "-p", "thrum-transport", "--no-default-features"], capture_output=True, text=True)
    licences = []
    for item in sorted(reached - {transport}):
        package = packages[item]
        licence = package.get("license")
        alternatives = [option.strip(" ()") for option in (licence or "").replace("/", " OR ").split(" OR ")]
        permissive = ["MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "ISC", "BSD-3-Clause", "BSD-2-Clause", "Zlib", "Unicode-3.0", "Unlicense", "CC0-1.0", "CDLA-Permissive-2.0"]
        chosen = next((option for option in alternatives if all(part.strip(" ()") in permissive for part in option.split(" AND "))), None)
        assert chosen, (package["name"], licence)
        licences.append({"name": package["name"], "version": package["version"], "license": licence, "selected_license": chosen})
    return {"core": core.strip(), "core_builds_alone": build_core.returncode == 0, "shipping": shipping, "existing_versions_preserved": len(baseline)-len(removed), "removed_since_baseline": removed,
            "crates_added_by_this_module": [f"{name} {version}" for name, version in added], "workspace_packages": closures, "adapter_licences": licences}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["conformance", "closure", "costs", "session", "hostile", "overload", "suspend"])
    parser.add_argument("--target", choices=["mac-metal", "chrome-webgpu", "ios-sim"], default="mac-metal")
    args = parser.parse_args()
    SCRATCH.mkdir(parents=True, exist_ok=True)
    output = SCRATCH / f"{args.command}-{args.target}-{time.time_ns()}"
    output.mkdir()
    record = {"commit": run(["git", "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip(), "source_dirty":bool(run(["git","status","--porcelain"],capture_output=True,text=True).stdout.strip()), "target": args.target, "machine": platform.platform(), "load": os.getloadavg()}
    if args.target == "ios-sim" or args.command == "session":
        record["simulator"] = {"device":SIM,"target_triple":"aarch64-apple-ios-sim","runtimes":json.loads(run(["xcrun","simctl","list","runtimes","--json"],capture_output=True,text=True).stdout)}
    if args.command == "closure":
        record["closure"] = closure(output)
    elif args.command in ["hostile", "overload"]:
        build("mac-metal")
        record[args.command] = in_process(args.command, output / args.command)
    elif args.command == "suspend":
        build("mac-metal")
        if args.target != "mac-metal":
            build(args.target)
        if args.target == "chrome-webgpu":
            record["suspend"] = chrome_suspend(output / "suspend")
        else:
            record["suspend"] = native_suspend(args.target, output / "suspend")
    elif args.command == "session":
        record["session"] = session(output)
    elif args.command == "costs":
        build("mac-metal")
        if args.target == "chrome-webgpu":
            build(args.target)
        record["runs"] = []
        for repetition in range(2):
            for controller in ["cubic","bbr"]:
                row = exchange(args.target,output / f"{controller}-{repetition}",True,seconds=90,controller=controller,case="costs")
                row["controller"] = controller
                row["repetition"] = repetition
                row["load"] = os.getloadavg()
                matching = [prior for prior in record["runs"] if prior["controller"] == controller]
                prior = matching[-1] if matching else None
                def result_cost(item):
                    client = item["client"]
                    return (client["result"] if isinstance(client,dict) else next(value["result"] for value in client if "result" in value))["costs"]
                costs = result_cost(row)
                def metrics(value):
                    result = {"connect_to_open_ms":value["connect_to_open_ms"],"wire_bytes_per_application_byte":value["wire_bytes_per_application_byte"]}
                    result.update({f"transfer_{index}_ms":number for index,number in enumerate(value["transfer_ms"])})
                    for phase in value["phases"]:
                        for lane in ["reliable_ms","datagram_ms"]:
                            result.update({f"{phase['phase']}_{lane}_{quantile}":phase[lane][quantile] for quantile in ["median","p95","maximum"]})
                    return result
                costs["quarter_worse_rows"] = [] if prior is None else [{"row":name,"current":value,"previous":metrics(result_cost(prior))[name]} for name,value in metrics(costs).items() if value > metrics(result_cost(prior))[name]*1.25]
                costs["quarter_worse_than_previous_run"] = None if prior is None else bool(costs["quarter_worse_rows"])
                if args.target == "mac-metal":
                    allocation = run(["cargo","bench","--locked","--offline","-p","link-proof","--features","allocation-proof","--bench","allocations"],capture_output=True,text=True,env={**ENV,"TRANSPORT_ALLOCATION_CONTROLLER":controller})
                    assert "warmed_send_and_drain" in allocation.stdout and "alloc" in allocation.stdout.lower()
                    allocation_path = output / f"allocations-{controller}-{repetition}.txt"
                    allocation_path.write_text(allocation.stdout+allocation.stderr)
                    row["allocation_report"] = str(allocation_path.relative_to(ROOT))
                record["runs"].append(row)
    else:
        build("mac-metal")
        if args.target != "mac-metal":
            build(args.target)
        record["direct"] = exchange(args.target, output / "direct", False)
        record["conditioned"] = exchange(args.target, output / "conditioned", True)
        for conditioned in [False, True]:
            for case in ["limits", "close-host", "close-client", "close-load"]:
                key = f"{case}-{'conditioned' if conditioned else 'direct'}"
                record[key] = exchange(args.target, output / key, conditioned, seconds=8, case=case)
        record["expiry"] = exchange(args.target, output / "expiry", True, seconds=6, case="expiry")
        record["idle-cut"] = exchange(args.target, output / "idle-cut", True, seconds=7, case="idle")
        for identity in ["hash", "version"]:
            record["wrong-" + identity] = exchange(args.target, output / ("wrong-" + identity), False, seconds=6, case="idle", identity=identity)
        record["startup-cut"] = exchange(args.target, output / "startup-cut", True, seconds=7, case="startup-cut")
        record["kill-peer"] = exchange(args.target, output / "kill-peer", True, seconds=7, case="kill-peer")
        record["reconnect"] = reconnect(args.target, output / "reconnect")
        if args.target == "mac-metal":
            # The native host runs on the Mac for every target; its teardown is proved once, here.
            record["host-drop"] = host_drop(output / "host-drop")
    (output / "record.json").write_text(json.dumps(record, indent=2) + "\n")
    print(output / "record.json")


def stop_on_signal(number, _frame):
    # Children run in their own sessions, so a runner's group kill doesn't reach them. Exiting through
    # SystemExit runs each proof's `finally`, which stops its hosts, relays, clients and browsers.
    for each in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(each, signal.SIG_IGN)
    raise SystemExit(128 + number)


if __name__ == "__main__":
    for number in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, stop_on_signal)
    main()
