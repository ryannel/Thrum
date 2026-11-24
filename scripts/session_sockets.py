"""Run the reference world on independent session peers over A0-conditioned sockets."""

from bisect import bisect_right
from contextlib import contextmanager
from dataclasses import dataclass, field
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import statistics
import subprocess
import sys
import tempfile
import time

import sim2d_conformance as conformance
import simulator_lifecycle
import transport_link as transport

ROOT = conformance.ROOT
SCRATCH = ROOT / "tmp/session-sockets"
SCENARIOS = ("steady", "faults", "slow", "listen", "costs")
MAC = "mac-metal"
CHROME = "chrome-webgpu"
IOS = "ios-sim"


def write_atomic(path, text):
    staging = path.with_suffix(".partial")
    staging.write_text(text)
    staging.replace(path)


@dataclass
class Log:
    path: Path
    position: int = 0
    records: list = field(default_factory=list)
    by_event: dict = field(default_factory=dict)

    def read(self):
        if self.path.exists():
            with self.path.open() as stream:
                stream.seek(self.position)
                while line := stream.readline():
                    if not line.endswith("\n"):
                        break
                    row = json.loads(line)
                    self.records.append(row)
                    self.by_event.setdefault(row.get("event"), []).append(row)
                    self.position = stream.tell()
        return self.records

    def events(self, event):
        self.read()
        return self.by_event.get(event, [])


def lock_args(resource):
    script = os.environ.get("ORCHESTRATE_SCRIPT")
    if not script:
        raise RuntimeError("Set ORCHESTRATE_SCRIPT to agent_loop.py for device admission")
    return [sys.executable, script, "lock", resource, "--"]


@dataclass
class Peer:
    target: str
    output: Path
    log: Log
    ticket: str
    process: subprocess.Popen | None = None
    lifecycle: list = field(default_factory=list)

    def command(self, value):
        write_atomic(self.output / "control", value)

    def page(self, value):
        write_atomic(self.output / "page", value)

    def playing(self):
        return [row for row in self.log.events("state") if row["state"] == "Playing"]

    def closed(self):
        return [row for row in self.log.events("state") if row["state"].startswith("Closed(")]


class Match:
    def __init__(self, directory, programs, bodies, ticks, controller="bbr", listen=False, axes=False,
                 join_ms=60_000, chrome_admitted=False):
        self.output = directory
        self.directory = directory
        self.programs = programs
        self.bodies = bodies
        self.ticks = ticks
        self.controller = controller
        self.listen = listen
        self.axes = axes
        self.join_ms = join_ms
        self.chrome_admitted = chrome_admitted
        self.processes = []
        self.peers = []
        self.harness = []
        self.host = Log(directory / "host.jsonl")
        self.config = None

    def process(self, command, stderr, stdout=None, *, append=False):
        with stderr.open("a" if append else "w") as errors:
            if stdout is None:
                process = subprocess.Popen([str(x) for x in command], cwd=ROOT, stderr=errors,
                                           stdout=errors, start_new_session=True)
            else:
                with stdout.open("a" if append else "w") as output:
                    process = subprocess.Popen([str(x) for x in command], cwd=ROOT, stderr=errors,
                                               stdout=output, start_new_session=True)
        self.processes.append((process, stderr))
        return process

    def wait(self, name, test, seconds=75):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            for peer in self.peers:
                errors = peer.log.events("error")
                if errors:
                    raise RuntimeError(f"{peer.output}: {errors[-1]}")
            value = test()
            if value:
                return value
            for process, stderr in self.processes:
                if process.poll() not in (None, 0) and not getattr(process, "expected_exit", False):
                    raise RuntimeError(f"process exited {process.returncode}: {stderr}")
            time.sleep(0.025)
        raise RuntimeError(f"Timed out waiting for {name}; see {self.directory}")

    def tick(self):
        commits = self.host.events("commit")
        return commits[-1]["tick"] if commits else 0

    def launch_host(self):
        self.directory.mkdir(parents=True, exist_ok=False)
        self.process([self.programs[MAC], "socket-host", self.bodies, self.ticks, self.controller,
                      str(self.listen).lower(), str(self.axes).lower(), self.join_ms, self.output / "host-control"],
                     self.output / "host.stderr", self.host.path)
        self.config = self.wait("host ticket", lambda: self.host.events("host"), 15)[0]

    def start(self):
        write_atomic(self.output / "host-control", "run")

    def end_after(self, ticks):
        end = self.tick() + ticks
        assert end < self.ticks
        self.ticks = end
        write_atomic(self.output / "host-control", f"end {end}")

    def launch(self, target, *, hold=False):
        index = len(self.peers)
        directory = self.output / f"player-{index}"
        directory.mkdir()
        version, host, fingerprint = self.config["ticket"].split("@")
        relay_log = directory / "relay-start.jsonl"
        self.process([sys.executable, "scripts/transport_relay.py", "--host", host, "--seed", 29721 + index,
                      "--control", directory / "relay-control", "--output", directory / "relay.json"],
                     directory / "relay.stderr", relay_log)
        relay = Log(relay_log)
        address = self.wait("relay", relay.read, 10)[0]["address"]
        peer = Peer(target, directory, Log(directory / "session.jsonl"), "@".join((version, address, fingerprint)))
        self.peers.append(peer)
        if hold:
            peer.command("hold")
        self.spawn(peer)
        return peer

    def spawn(self, peer):
        axes = str(self.axes).lower()
        if peer.target == CHROME:
            admission = [] if self.chrome_admitted else lock_args("chrome")
            command = [*admission, "node", "scripts/session_socket_browser.mjs",
                       peer.output / "chrome", self.programs[CHROME], peer.log.path, peer.output / "control",
                       peer.output / "page", peer.ticket, self.bodies, axes]
            peer.process = self.process(command, peer.output / "peer.stderr")
        else:
            command = [self.programs[peer.target], "socket-client", peer.ticket, self.bodies, self.controller,
                       peer.output / "control", peer.output / "token", axes]
            if peer.target == IOS:
                command = simulator_lifecycle.lock_command([sys.executable, __file__, "simulator-peer",
                                                            peer.output / "restart", *command])
                if simulator_lifecycle.admission()[0]:
                    raise RuntimeError("Run the socket match without an outer simulator lock")
            peer.process = self.process(command, peer.output / "peer.stderr", peer.log.path, append=True)

    def action(self, action, **fields):
        event = {"action": action, "wall_us": time.time_ns() // 1000, "host_tick": self.tick(), **fields}
        self.harness.append(event)
        write_atomic(self.output / "harness.json", json.dumps(self.harness, indent=2) + "\n")
        return event

    def host_peer(self, playing, until=None):
        return next(row for row in reversed(self.host.events("stream")) if row["epoch"] == playing["epoch"]
                    and (until is None or row["wall_us"] <= until))

    def after_playing(self, peer, mark, *, epoch=None, stay=False):
        def found():
            return next((row for row in peer.playing() if row["wall_us"] > mark["wall_us"]
                         and (epoch is None or (row["epoch"] == epoch) == stay)), None)

        playing = self.wait(f"{peer.target} to play", found, self.join_ms / 1000 + 20)
        self.wait("digest after playing", lambda: any(
            row["wall_us"] > playing["wall_us"] and row["tick"] >= playing["tick"] + 30
            for row in peer.log.events("digest")))
        return playing

    def terminate_process(self, peer):
        pid = peer.log.events("client")[-1]["pid"]
        if peer.target != IOS:
            peer.process.expected_exit = True
        os.kill(pid, signal.SIGKILL)
        if peer.target != IOS:
            peer.process.wait(timeout=10)
        return pid

    def finish(self, expected):
        for peer in expected:
            def final_state():
                states = peer.log.events("state")
                if states and states[-1]["state"].startswith(("Ended(", "Closed(", "Refused(")):
                    return states[-1]
                return None

            row = self.wait(f"{peer.target} final endpoint", final_state, 1500)
            assert row["state"].startswith("Ended(") and row["tick"] == self.ticks, row
        self.wait("host finish", lambda: self.host.events("finish"), 30)

    def close(self):
        ios = {peer.process: peer for peer in self.peers if peer.target == IOS and peer.process}
        for process, _ in reversed(self.processes):
            if process in ios:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=12)
                peer = ios[process]
                peer.lifecycle = [json.loads(line) for line in (peer.output / "peer.stderr").read_text().splitlines()
                                  if line.startswith('{"event":')]
            else:
                transport.stop(process)
        for peer in self.peers:
            profile = peer.output / "chrome"
            if profile.exists():
                shutil.rmtree(profile)
            (peer.output / "token").unlink(missing_ok=True)
            (peer.output / "token.partial").unlink(missing_ok=True)


@contextmanager
def match(*args, **kwargs):
    instance = Match(*args, **kwargs)
    try:
        instance.launch_host()
        yield instance
    finally:
        instance.close()


def simulator_peer(args):
    restart, *command = args
    restart = Path(restart)
    with simulator_lifecycle.supervise() as turn:
        while True:
            result = turn.run(["xcrun", "simctl", "spawn", turn.udid, *command], check=False)
            if not restart.exists():
                return result.returncode
            deadline = time.monotonic() + 35
            while restart.read_text() != "restart" and time.monotonic() < deadline:
                turn.check()
                time.sleep(0.025)
            if restart.read_text() != "restart":
                return result.returncode
            restart.unlink()
            turn.event("peer_restart")


def compare(run, peer, *, finish=True, expected_faults=0):
    host = {row["tick"]: row["digest"] for row in run.host.events("digest")}
    images = {(row["tick"], row["digest"], row["bytes"]) for row in run.host.events("image")}
    interval = run.config["digest_interval"]
    next_tick = None
    last_tick = None
    mismatches = []
    baselines = []
    checked = 0
    for row in peer.log.read():
        if row["event"] == "baseline":
            assert (row["tick"], row["digest"], row["bytes"]) in images, (peer.target, "baseline differs", row)
            baselines.append(row)
            next_tick = (row["tick"] // interval + 1) * interval
        if row["event"] == "digest":
            assert row["tick"] == next_tick, (peer.target, "missing digest", next_tick, row)
            last_tick = row["tick"]
            next_tick += interval
            assert last_tick in host
            if row["digest"] != host[last_tick]:
                mismatches.append(row)
            checked += 1
    assert baselines and checked > 0, peer.target
    assert len(mismatches) == expected_faults, (peer.target, "digest mismatch", mismatches)
    if finish:
        assert last_tick == run.ticks // interval * interval, (peer.target, "final digest", last_tick)
    return {"baselines": baselines, "digests_checked": checked, "mismatches": mismatches, "last_digest_tick": last_tick}


def longest_gap(commits, since=None):
    return max((b["clock_us"] - a["clock_us"] for a, b in zip(commits, commits[1:])
                if since is None or since[0] <= b["wall_us"] <= since[1]), default=None)


def relay_record(peer):
    record = json.loads((peer.output / "relay.json").read_text())
    assert record["one_way_ms"] == 50 and record["jitter_ms"] == 10 and record["loss"] == 0.02
    assert all(row["forwarded"] > 0 for row in record["directions"])
    return record


def report(run, peers):
    commits = run.host.events("commit")
    assert [row["tick"] for row in commits] == list(range(1, run.ticks + 1))
    digests = run.host.events("digest")
    assert [row["tick"] for row in digests] == list(range(run.config["digest_interval"], run.ticks + 1,
                                                         run.config["digest_interval"]))
    identities = [row for peer in run.peers for row in peer.log.events("client")]
    all_pids = [run.config["pid"], *(row["pid"] for row in identities)]
    assert len(set(all_pids)) == len(all_pids), all_pids
    warm = commits[600:]
    admitted_fraction = [
        sum(row["admissions"][slot] for row in warm) / len(warm)
        for slot in range(len(run.peers) + run.listen)
    ]
    started, = run.host.events("start")
    return {"host": run.config, "host_commits": len(commits), "admitted_fraction_after_warmup": admitted_fraction,
            "host_digests": len(digests), "host_held_ticks": commits[-1]["held"],
            "elapsed_host_us": commits[-1]["clock_us"] - started["clock_us"],
            "longest_host_commit_gap_us": longest_gap(commits),
            "harness_events": run.harness, "host_log": "host.jsonl",
            "players": [{"target": peer.target, "identity": peer.log.events("client"),
                         "log": str(peer.log.path.relative_to(run.output)), "relay": relay_record(peer),
                         **({"simulator_turn": peer.lifecycle} if peer.target == IOS else {}),
                         **result} for peer, result in peers]}


def stable(run, targets):
    peers = []
    for target in targets:
        peer = run.launch(target)
        run.wait(f"{peer.target} target admission", lambda: peer.log.events("client"), 2100)
        run.wait(f"{peer.target} initial baseline", lambda: peer.log.events("baseline"))
        peers.append(peer)
    run.action("start")
    run.start()
    run.finish(peers)
    if run.listen:
        assert all(row["admissions"][0] for row in run.host.events("commit")), "listen host missed its own frame"
    results = []
    for peer in peers:
        proof = compare(run, peer)
        assert [baseline["tick"] for baseline in proof["baselines"]] == [0]
        results.append((peer, proof))
    return results


def returned(run, peer, before, report_at, action, *, stay=False):
    if stay:
        restored = run.wait("repair baseline", lambda: next((row for row in peer.log.events("baseline")
            if row["epoch"] == before["epoch"] and row["wall_us"] > report_at["wall_us"]), None))
        mark = {"wall_us": restored["wall_us"] - 1000}
    else:
        mark = report_at
    new = run.after_playing(peer, mark, epoch=before["epoch"], stay=stay)
    assert new["slot"] == before["slot"], (before, new)
    if not stay:
        assert new["used_saved_token"] and new["same_token"], new
        if action in ("cut and return", "return after absence"):
            assert new["link"] != before["link"], (new, before)
    old_host = run.host_peer(before, report_at["wall_us"])
    new_host = run.host_peer(new)
    if not stay:
        assert old_host["link"] != new_host["link"]
    baselines = [row for row in peer.log.events("baseline")
                 if row["epoch"] == new["epoch"] and row["wall_us"] > report_at["wall_us"]]
    assert baselines or stay
    ends = [row for row in run.host.events("disconnect") if row["link"] == old_host["link"]]
    finishes = [row for row in peer.closed() if row["epoch"] == before["epoch"]]
    return {"event": action, "trigger": report_at, "before": before, "after": new, "baselines": baselines,
            "authority_before": old_host, "authority_after": new_host,
            "authority_report": ends, "replica_report": finishes}


def witnesses(run):
    peers = [run.launch(MAC), run.launch(MAC)]
    for peer in peers:
        run.wait("witness initial baseline", lambda: peer.log.events("baseline"))
    return peers


def faults(run, target):
    others = witnesses(run)
    peer = None
    if target != MAC:
        peer = run.launch(target, hold=True)
        run.wait(f"{target} target admission", lambda: peer.log.events("client"), 2100)
    run.start()
    run.wait("late-join threshold", lambda: run.tick() > 3600, 180)
    joining = run.action("join")
    if peer:
        peer.command("run")
    else:
        peer = run.launch(target)
    playing = run.after_playing(peer, joining)
    baseline = peer.log.events("baseline")[0]
    assert baseline["tick"] > 3600
    timeline = [{"event": "late join", "trigger": joining, "after": playing, "baselines": [baseline],
                 "authority_after": run.host_peer(playing)}]

    cut = run.action("cut relay")
    write_atomic(peer.output / "relay-control", "cut")
    host_link = run.host_peer(playing)["link"]
    run.wait("host timeout", lambda: any(row["link"] == host_link for row in run.host.events("disconnect")))
    run.wait("client timeout", peer.closed)
    run.wait("cut longer than the idle timeout",
             lambda: time.time_ns() // 1000 - cut["wall_us"] > (run.config["idle_ms"] + 500) * 1000)
    heal = run.action("heal relay")
    write_atomic(peer.output / "relay-control", "open")
    peer.command("reconnect")
    first = returned(run, peer, playing, heal, "cut and return")
    assert first["authority_report"][0]["reason"] == "TimedOut", first
    assert first["replica_report"], first
    first["cut"] = cut
    timeline.append(first)

    prior = first["after"]
    old_host = run.host_peer(prior)["link"]
    interrupted = run.action("reload" if target == CHROME else "kill")
    if target == CHROME:
        peer.command("hold")
        peer.page("reload")
        run.wait("Chrome reload acknowledgement", lambda: any(
            row["command"] == "reload" for row in peer.log.events("page")))
    else:
        if target == IOS:
            write_atomic(peer.output / "restart", "armed")
        interrupted["pid"] = run.terminate_process(peer)
        peer.command("run")
    run.wait("host observed the lost link", lambda: any(
        row["link"] == old_host for row in run.host.events("disconnect")))
    if target == CHROME:
        peer.command("run")
    else:
        run.action("restart")
        if target == IOS:
            write_atomic(peer.output / "restart", "restart")
        else:
            run.spawn(peer)
    second = returned(run, peer, prior, interrupted, "reload or process restart")
    assert second["authority_report"]
    if target != CHROME:
        assert len(peer.log.events("client")) == 2
    timeline.append(second)

    perturbation = run.action("perturb carried resource")
    peer.command("perturb")
    repaired = returned(run, peer, second["after"], perturbation, "repair", stay=True)
    diverged, = run.host.events("divergence")
    assert diverged["link"] == repaired["authority_after"]["link"] and 3 in diverged["parts"], diverged
    assert len(repaired["baselines"]) == 1 and repaired["authority_after"]["repairs"] == 1, repaired
    repaired.update(divergence=diverged, changed=peer.log.events("perturbed"))
    timeline.append(repaired)

    run.finish([*others, peer])
    subject = compare(run, peer, expected_faults=1)
    assert subject["mismatches"][0]["digest"] == diverged["actual"]
    assert len(subject["baselines"]) == 4
    return [(other, compare(run, other)) for other in others] + [(peer, dict(subject, fault_events=timeline))]


def capped(run, rows):
    assert rows
    maximum = {
        key: max(row[key] for row in rows)
        for key in ("retained_sets", "retained_bytes", "staged_frames")
    }
    # A set fits one reliable message. Commands carry at most 256 payload bytes plus their wire headers.
    limits = {
        "retained_sets": run.config["backlog"],
        "retained_bytes": run.config["backlog"] * 65_536 + run.config["image_limit"]
        + run.config["command_cap"] * 320,
        "staged_frames": run.config["staging"],
    }
    assert all(maximum[key] <= limit for key, limit in limits.items()), (maximum, limits)
    queues = [max(row["counters"]["outbound"][lane] for row in rows) for lane in (0, 1)]
    assert all(value <= cap for value, cap in zip(queues, run.config["outbound_caps"], strict=True))
    return {"observed_session_maxima": maximum, "session_limits": limits,
            "outbound_maxima": queues, "outbound_caps": run.config["outbound_caps"]}


def slow_mac(run):
    others = witnesses(run)
    peer = run.launch(MAC)
    run.wait("initial baseline", lambda: peer.log.events("baseline"))
    run.start()
    before = run.after_playing(peer, {"wall_us": 0})
    run.wait("pre-stall commits", lambda: run.tick() >= 300)
    host = run.host_peer(before)
    stopped = run.action("stop taking messages")
    peer.command("stall")
    closed = run.wait("authority closed stalled link", lambda: next(
        (row for row in run.host.events("disconnect") if row["link"] == host["link"]), None))
    assert closed["reason"] == "Local(6)", closed
    peer.command("run")
    replica_close = run.wait("stalled client observed close", peer.closed)
    run.finish(others)
    commits = run.host.events("commit")
    during = (stopped["wall_us"], closed["wall_us"])
    rows = [row for row in run.host.events("stream") if row["link"] == host["link"]
            and during[0] <= row["wall_us"] <= during[1]]
    assert rows[-1]["held"] == rows[-2]["held"] < closed["tick"]
    info = {"stall": stopped, "authority_close": closed, "replica_close": replica_close,
            "longest_commit_gap_during_stall_us": longest_gap(commits, during), **capped(run, rows)}
    return [(other, compare(run, other)) for other in others] + [(peer, dict(compare(run, peer, finish=False), **info))]


def browser_lifecycle(run, peer, state):
    offset = len(peer.log.events("page"))
    action = run.action(state)
    peer.page(state)
    ack = run.wait(f"Chrome page {state}", lambda: next(
        (row for row in peer.log.events("page")[offset:] if row["command"] == state), None))
    return {**action, "browser_ack": ack}


def slow_chrome(run):
    others = witnesses(run)
    peer = run.launch(CHROME)
    run.wait("Chrome target admission", lambda: peer.log.events("client"), 2100)
    run.wait("initial baseline", lambda: peer.log.events("baseline"))
    run.start()
    before = run.after_playing(peer, {"wall_us": 0})
    run.wait("pre-freeze commits", lambda: run.tick() >= 300)
    authority = run.host_peer(before)
    slot, link = before["slot"], authority["link"]
    run.wait("fresh non-neutral controls before freezing", lambda:
             run.host.events("axes")[-1]["axes"][slot] != [0.0] * 3
             and run.tick() - run.host_peer(before)["last_staged"] < run.config["held_expiry"] // 2)

    short = browser_lifecycle(run, peer, "frozen")
    neutral = run.wait("frozen player's neutral axes", lambda: next(
        (row for row in run.host.events("axes") if row["wall_us"] >= short["browser_ack"]["wall_us"]
         and row["axes"][slot] == [0.0] * 3), None))
    run.wait("short freeze after held-control expiry",
             lambda: run.tick() >= short["host_tick"] + run.config["held_expiry"] + 60)
    thaw = browser_lifecycle(run, peer, "active")
    assert not [row for row in run.host.events("disconnect") if row["link"] == link]
    run.wait("the same link resumed authoring", lambda: run.host_peer(before)["last_input"] > neutral["tick"] + 30)
    for witness in [*others, peer]:
        row = run.wait("peer committed neutral controls", lambda: next(
            (row for row in witness.log.events("axes") if row["tick"] == neutral["tick"]), None))
        assert row["axes"][slot] == [0.0] * 3
    short_seen = [row for row in run.host.events("stream") if row["link"] == link
                  and short["browser_ack"]["wall_us"] <= row["wall_us"] <= thaw["browser_ack"]["wall_us"]]
    assert short_seen and any(row["last_staged"] + run.config["held_expiry"] <= neutral["tick"] for row in short_seen)
    resumed = run.host_peer(before)

    long = browser_lifecycle(run, peer, "frozen")
    closed = run.wait("authority closed frozen link", lambda: next(
        (row for row in run.host.events("disconnect") if row["link"] == link), None))
    assert closed["reason"] == "Local(6)", closed
    run.wait("long freeze past the absence limit",
             lambda: run.tick() > long["host_tick"] + run.config["absence"] + 30)
    long_thaw = browser_lifecycle(run, peer, "active")
    run.wait("thawed client observed close", peer.closed)
    reconnect = run.action("reconnect after long freeze")
    peer.command("reconnect")
    comeback = returned(run, peer, before, reconnect, "return after absence")
    run.finish([*others, peer])
    long_seen = [row for row in run.host.events("stream") if row["link"] == link
                 and long["browser_ack"]["wall_us"] <= row["wall_us"] <= closed["wall_us"]]
    assert long_seen and long_seen[-1]["last_input"] + run.config["absence"] == closed["tick"]
    subject = compare(run, peer)
    subject["freezes"] = {
        "short": {"freeze": short, "thaw": thaw, "neutral_commit": neutral, "resumed": resumed,
                  "authority_samples": short_seen},
        "long": {"freeze": long, "thaw": long_thaw, "authority_samples": long_seen, "return": comeback},
    }
    return [(other, compare(run, other)) for other in others] + [(peer, subject)]


def cost_rows(run, peer):
    playing = peer.playing()[0]
    baseline, = peer.log.events("baseline")
    commit = run.host.events("commit")
    wall = [row["wall_us"] for row in commit]
    samples = [row for row in peer.log.events("sample") if row["playout"] is not None and row["lead"] is not None]
    assert samples and baseline["hello_to_restore_us"] is not None and playing["hello_to_playing_us"] is not None
    delays = [(bisect_right(wall, row["wall_us"]) - row["playout"]) * run.config["tick_us"] for row in samples]
    controller_samples = [row for row in samples if row["prediction"] is not None]
    assert controller_samples
    predicted_samples = [row for row in controller_samples if row["prediction"]["predicted"]]
    # A confirmed controller and the crowd present the same endpoint. A zero
    # measured offset is possible even though a browser's wire counter is absent.
    hero_leads = [row["prediction"]["tick"] - row["playout"] if row["prediction"]["predicted"] else 0
                  for row in controller_samples]
    stalls = peer.log.events("stall")
    end = next(row for row in peer.log.events("state") if row["state"].startswith("Ended("))
    authority_link = run.host_peer(playing)["link"]
    authority = next(row["counters"] for row in run.host.events("disconnect") if row["link"] == authority_link)
    client = end["counters"]
    relay = relay_record(peer)
    duration = (commit[-1]["wall_us"] - commit[0]["wall_us"] + run.config["tick_us"]) / 1_000_000
    held = commit[-1]["held"][playing["slot"]]
    wire = [row["seen_bytes"] for row in relay["directions"]]
    images = run.host.events("image")
    image_tick_us = max(row["advance_us"] for row in images)
    row = {
        "to_host": {"application_bytes_sent": client["application_sent"],
                    "application_bytes_delivered": authority["application_received"],
                    "endpoint_wire_bytes": client["wire_sent"], "relay_wire_bytes": wire[0]},
        "from_host": {"application_bytes_sent": authority["application_sent"],
                      "application_bytes_delivered": client["application_received"],
                      "endpoint_wire_bytes": authority["wire_sent"], "relay_wire_bytes": wire[1]},
        "browser_wire_bytes_absent": peer.target == CHROME and client["wire_sent"] is None,
        "baseline_bytes": baseline["bytes"],
        "join_total_us": playing["hello_to_playing_us"],
        "join_hello_to_welcome_us": baseline["hello_to_restore_us"] - baseline["welcome_to_restore_us"],
        "join_transfer_us": baseline["welcome_to_restore_us"],
        "join_catch_up_us": playing["hello_to_playing_us"] - baseline["hello_to_restore_us"],
        "confirmed_delay_median_us": statistics.median(delays),
        "confirmed_delay_max_us": max(delays),
        "lead_median_ticks": statistics.median(sample["lead"] for sample in samples),
        "lead_range_ticks": [min(sample["lead"] for sample in samples), max(sample["lead"] for sample in samples)],
        "playout_stall_count": len(stalls),
        "playout_stall_total_us": sum(stall["duration_us"] for stall in stalls),
        "playout_stalls": stalls,
        "corrections_per_minute": end["corrections"] * 60 / duration,
        "hero_lead_over_crowd_median_ticks": statistics.median(hero_leads),
        "hero_controller_samples": len(controller_samples),
        "hero_predicted_samples": len(predicted_samples),
        "ticks_held": held,
        "longest_host_commit_gap_us": longest_gap(commit),
        "image_tick_us": image_tick_us,
        "match_seconds": duration,
        "tripwires": [],
    }
    for direction in ("to_host", "from_host"):
        for layer in ("relay_wire_bytes", "application_bytes_sent"):
            if row[direction][layer] * 8 / duration > 64_000:
                row["tripwires"].append({"name": "64 kbit/s", "direction": direction, "layer": layer})
    if image_tick_us > run.config["tick_us"]:
        row["tripwires"].append({"name": "image write longer than tick period"})
    return row


def inspect_costs(result, previous):
    if previous:
        for key, value in result.items():
            if isinstance(value, (int, float)) and not isinstance(value, bool) and key in previous:
                if value > previous[key] * 1.25:
                    result["tripwires"].append({"name": "quarter growth", "field": key,
                                                "previous": previous[key], "sample": value})
        for direction in ("to_host", "from_host"):
            for key, value in result[direction].items():
                if value is not None and value > previous[direction][key] * 1.25:
                    result["tripwires"].append({"name": "quarter growth", "field": f"{direction}.{key}",
                                                "previous": previous[direction][key], "sample": value})


class IncompleteCostJoin(RuntimeError):
    """A transport or session ended before it supplied the join measurement."""


def cost_match(directory, programs, target, controller, bodies, chrome_admitted):
    with match(directory, programs, bodies, 10_000, controller, join_ms=120_000,
               chrome_admitted=chrome_admitted) as run:
        peer = run.launch(target)
        run.wait(f"{target} target admission", lambda: peer.log.events("client"), 2100)

        def live(value):
            states = peer.log.events("state")
            if states and states[-1]["state"].startswith(("Closed(", "Refused(")):
                raise IncompleteCostJoin(states[-1]["state"])
            return value()

        # Measure an initial join with the normal lobby start. The faults
        # campaign independently forces joins into an already running match.
        deadline = run.join_ms / 1000 + 20
        run.wait(f"{target} cost baseline", lambda: live(lambda: peer.log.events("baseline")), deadline)
        run.start()
        run.wait(f"{target} cost join outcome", lambda: live(peer.playing), deadline)
        run.end_after(900)
        run.finish([peer])
        results = [(peer, compare(run, peer))]
    measurement = cost_rows(run, peer)
    measurement["join_context"] = "initial lobby"
    return measurement, report(run, results)


def costs(directory, programs, target, *, chrome_admitted=False):
    rows = []
    prior = {}
    for repetition in (1, 2):
        for controller in ("bbr", "cubic"):
            smaller = None
            for bodies in (2000, 10000):
                load = os.getloadavg()
                stem = f"{controller}-{repetition}-{bodies}"
                for attempt in range(1, 4):
                    path = directory / (stem if attempt == 1 else f"{stem}-attempt-{attempt}")
                    try:
                        record, receipt = cost_match(path, programs, target, controller, bodies, chrome_admitted)
                        break
                    except IncompleteCostJoin as error:
                        if attempt == 3:
                            raise
                        print(f"{target} {path.name} ended before playing ({error}); retrying", flush=True)
                record.update(controller=controller, repetition=repetition, bodies=bodies, load_start=load,
                              load_end=os.getloadavg(), run=path.name, attempt=attempt, **receipt)
                inspect_costs(record, prior.get((controller, bodies)))
                prior[controller, bodies] = record
                if smaller and (record["join_total_us"] * smaller["baseline_bytes"]
                                > smaller["join_total_us"] * record["baseline_bytes"]):
                    record["tripwires"].append({"name": "join time grew faster than image", "smaller_row": smaller["run"]})
                smaller = record
                rows.append(record)
                write_atomic(path / "cost-row.json", json.dumps(record, indent=2) + "\n")
    return {"cost_rows": rows}


def run_scenario(scenario, target, directory, programs):
    if scenario == "costs":
        return costs(directory, programs, target)
    if scenario == "slow" and target == MAC:
        with match(directory / "unstalled", programs, 10_000, 1500, axes=True) as control:
            peers = stable(control, [MAC, MAC, MAC])
        other = report(control, peers)
        with match(directory / "stalled", programs, 10_000, 1500, axes=True) as run:
            peers = slow_mac(run)
        return {"stalled": report(run, peers), "unstalled": other}

    ticks = {"steady": 36_000, "listen": 3600, "faults": 7800, "slow": 2100}[scenario]
    with match(directory / "match", programs, 10_000, ticks, listen=scenario == "listen",
               axes=scenario == "slow") as run:
        if scenario == "steady":
            peers = stable(run, [MAC, MAC, CHROME, IOS])
        elif scenario == "listen":
            peers = stable(run, [CHROME, IOS])
        elif scenario == "faults":
            peers = faults(run, target)
        else:
            peers = slow_chrome(run)
    result = report(run, peers)
    if scenario in ("steady", "listen"):
        assert min(result["admitted_fraction_after_warmup"]) >= 0.95, result["admitted_fraction_after_warmup"]
    if scenario == "steady":
        assert result["elapsed_host_us"] >= 600_000_000
    return result


def file_sha(path):
    with path.open("rb") as data:
        return hashlib.file_digest(data, "sha256").hexdigest()


def proof(scenario, target, output, build):
    for number in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, interrupt)
    targets = {MAC, target}
    if scenario in ("steady", "listen"):
        targets.update((CHROME, IOS))
    SCRATCH.mkdir(parents=True, exist_ok=True)
    if output is None:
        output = SCRATCH / f"{scenario}-{target}-{time.time_ns()}" / "record.json"
    output = output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    release = {}
    provenance = conformance.identity()
    paths = {}
    target_settings = {MAC: "aarch64-apple-darwin", CHROME: "wasm32-unknown-unknown", IOS: "aarch64-apple-ios-sim"}
    with tempfile.TemporaryDirectory(prefix="build-", dir=SCRATCH) as temp:
        for platform in sorted(targets):
            built = build(platform, sockets=True)
            snapshot = Path(temp) / platform
            snapshot.mkdir()
            artifact = snapshot / built.name
            shutil.copy2(built, artifact)
            record = {"release": True, "hash_sha256": file_sha(artifact),
                      "target_triple": target_settings[platform]}
            if platform == CHROME:
                program = Path(temp) / "web"
                subprocess.run([Path.home() / ".cargo/bin/wasm-bindgen", "--target", "web",
                                "--out-dir", program, artifact], check=True)
                shutil.copyfile(ROOT / "examples/match-proof/socket_player.js", program / "socket_player.js")
                record["loaded_wasm_sha256"] = file_sha(program / "match_proof_bg.wasm")
                paths[platform] = program
            else:
                paths[platform] = artifact
            release[platform] = record
        began = time.monotonic()
        if scenario == "costs" and target == CHROME:
            result = chrome_cost_campaign(output.parent, paths, Path(temp) / "cost-rows.json")
        else:
            result = run_scenario(scenario, target, output.parent, paths)
        if IOS in targets:
            peer, = [peer for peer in result["players"] if peer["target"] == IOS]
            events = peer["simulator_turn"]
            boot, = [event for event in events if event["event"] == "booted"]
            shutdown, = [event for event in events if event["event"] == "shutdown_verified"
                         and event["utc_s"] > boot["utc_s"]]
            assert shutdown["udid"] == boot["udid"]
            release[IOS].update(runtime=simulator_lifecycle.RUNTIME, simulator=boot["udid"],
                                device=simulator_lifecycle.NAME)
        provenance["duration_seconds"] = time.monotonic() - began
    packet = {**provenance, "scenario": scenario, "target": target, "release": release, **result, "result": "pass"}
    write_atomic(output, json.dumps(packet, indent=2) + "\n")
    print(json.dumps({"result": "pass", "record": str(output)}))


def chrome_cost_campaign(directory, programs, record):
    # Builds finish before admission. Every row starts a fresh Chrome process
    # and measures an independent match on the same admitted target.
    command = [*lock_args("chrome"), sys.executable, __file__, "chrome-costs",
               directory, record, programs[MAC], programs[CHROME]]
    process = subprocess.Popen([str(arg) for arg in command], cwd=ROOT)
    try:
        status = process.wait()
        if status:
            raise subprocess.CalledProcessError(status, command)
    finally:
        transport.stop(process)
    return json.loads(record.read_text())


def interrupt(number, _frame):
    raise SystemExit(128 + number)


if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "simulator-peer":
        raise SystemExit(simulator_peer(sys.argv[2:]))
    if len(sys.argv) == 6 and sys.argv[1] == "chrome-costs":
        for number in (signal.SIGTERM, signal.SIGHUP):
            signal.signal(number, interrupt)
        directory, record, native, browser = map(Path, sys.argv[2:])
        result = costs(directory, {MAC: native, CHROME: browser}, CHROME, chrome_admitted=True)
        write_atomic(record, json.dumps(result))
        raise SystemExit(0)
    raise SystemExit("use scripts/session_match.py sockets")
