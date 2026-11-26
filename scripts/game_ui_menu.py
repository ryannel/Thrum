"""Qualify the V1 menu with scripted input through the live macOS window."""

import json
from itertools import groupby
import os
from pathlib import Path
import subprocess

from game_ui_status import prepare_store, wake_window_display

CARDS = [110, 120, 130, 140]
TICKET = "THRUM-ALPHA-29406-A82EXACL79FTG2OXQ4X65SPPDLX2OUF6KR18PJH5902C01W64NFQVE82NHXQ33DM0-COAST-8432"


def key(code, down=True, after=2, **options):
    return dict(action="key", code=code, down=down, after=after, **options)


def tap(code, after=2, **options):
    return [key(code, after=after, **options), key(code, down=False, **options)]


def observe(name, after=3):
    return dict(action="observe", name=name, after=after)


def click(control):
    return [dict(action="cursor", control=control), dict(action="button"),
            dict(action="button", down=False, after=4)]


def choice():
    return tap("KeyM") + [observe("armed", after=24)]


def route_script():
    steps = [observe("initial"), key("KeyD"), observe("walking", after=10)]
    for index, card in enumerate(CARDS):
        if index:
            steps.extend([key("KeyD")] if index == 1 else [])
        steps.extend(choice())
        if index == 0:
            steps.extend([observe("stopped-a", after=8), observe("stopped-b", after=8)])
        if index & 1:
            steps.extend(tap("ArrowRight"))
        if index & 2:
            steps.extend(tap("ArrowDown"))
        steps.extend([observe(f"focus-{card}"), *tap("Enter")])
        if index == 0:
            steps.extend([observe("resumed-a", after=8), observe("resumed-b", after=10),
                          key("KeyD", down=False)])
        if index == 1:
            steps.append(key("KeyD", down=False))
    for card in CARDS:
        if card == CARDS[0]:
            steps.extend([key("KeyM"), key("ArrowRight", after=1), key("KeyM", down=False),
                          observe("early-direction-armed", after=24), *click(card),
                          observe("early-direction-resumed", after=6), key("ArrowRight", down=False)])
        else:
            steps.extend([*choice(), *click(card)])
    return steps + [observe("complete", after=8)]


def drops_script():
    return [
        observe("launch", after=8), *tap("Escape"),
        *tap("KeyH"), observe("hosting", after=8), *tap("Enter"), observe("copied", after=5),
        *tap("Escape"), *tap("KeyJ"), observe("joining", after=8),
        *tap("KeyV", command=True), observe("pasted", after=8), *tap("Escape"),
        *choice(), *tap("ArrowRight"), observe("before-recovery"),
        dict(action="recover"), observe("after-recovery", after=14), *tap("Escape"),
        observe("complete", after=8),
    ]


def costs_script():
    return [
        observe("initial"), *tap("KeyE"), observe("edit"), *choice(), *tap("ArrowRight"),
        observe("key-effect"), dict(action="cursor", control=130),
        dict(action="button"), observe("pointer-effect", after=4),
        dict(action="button", down=False),
        *choice(), dict(action="touch", id=8, control=140),
        observe("touch-effect"), dict(action="touch", id=8, control=140, down=False),
        observe("complete", after=8),
    ]


def snapshots(record):
    return {entry["step"]["name"]: entry for entry in record["steps"]
            if entry["step"]["action"] == "observe"}


def effects(report, kind):
    return [row["effect"][kind] for row in report["effects"] if kind in row["effect"]]


def validate_route(record):
    marks = snapshots(record)
    report = record["final"]
    assert report["activations"] == [2] * 4, report["activations"]
    assert [marks[f"focus-{card}"]["before"]["focus"] for card in CARDS] == CARDS
    assert marks["walking"]["before"]["position"][0] > marks["initial"]["before"]["position"][0]
    assert marks["stopped-a"]["before"]["position"] == marks["stopped-b"]["before"]["position"]
    intervals = [list(rows) for _, rows in groupby(enumerate(report["choice_frames"]),
                                                    key=lambda row: row[1][0] - row[0])]
    assert len(intervals) == 8, [len(interval) for interval in intervals]
    for interval in intervals:
        positions = [item[1][1] for item in interval]
        assert len(positions) >= 2 and all(pos == positions[0] for pos in positions), interval
    assert marks["resumed-b"]["before"]["position"][0] > marks["resumed-a"]["before"]["position"][0]
    opened = [row["ms"] for row in report["effects"] if row["effect"].get("scope_opened") == 100]
    interrupted = [row["ms"] for row in report["effects"] if row["effect"].get("interrupted")]
    assert len(opened) == 8 and all(interrupted.count(time) == 1 for time in opened)
    assert any("KeyD" in contacts[0] for contacts in effects(report, "resumed") if contacts)
    armed = marks["early-direction-armed"]["before"]
    resumed = marks["early-direction-resumed"]["before"]
    assert armed["focus"] == CARDS[0]
    assert resumed["position"][0] > armed["position"][0]
    early_press, = [row for row in record["steps"] if row["step"].get("code") == "ArrowRight"
                    and row["step"].get("after") == 1]
    assert early_press["before"]["disarmed"]
    assert any("ArrowRight" in contact for contacts in effects(resumed, "resumed") for contact in contacts)
    assert all(row["input"].startswith("Release(Key(ArrowRight)")
               or not any(word in row["input"] for word in ["Arrow", "Enter", "Pointer"])
               for row in report["inputs"])


def validate_drops(record):
    marks = snapshots(record)
    launch = marks["launch"]["before"]
    report = record["final"]
    assert launch["ticket"] == TICKET
    assert any(row["effect"] == {"source": "Launch", "text": TICKET} for row in report["effects"])
    assert effects(report, "copy") == [TICKET]
    assert [row["access"] for row in report["clipboard"]] == ["Write", "Read"]
    if report["clipboard"][1]["allowed"]:
        assert any(row["effect"] == {"source": "Paste", "text": TICKET} for row in report["effects"])
    before, after = (marks[name] for name in ["before-recovery", "after-recovery"])
    assert after["device"] > before["device"]
    assert before["before"]["focus"] == after["before"]["focus"] == 120
    assert before["before"]["drawn"] == after["before"]["drawn"] == "Choice"


def validate_costs(report):
    required = {"presentation edit", "key press", "pointer press", "touch press"}
    for row in report["timing"]:
        assert 0 <= row["route_ms"] < float("inf")
        if "frames" in row:
            assert row["frames"] >= 1
            required.discard(row["kind"])
    assert not required, required


def run(root, destination, scratch, revision, environment, mode, load, digest):
    subprocess.run(["cargo", "build", "--release", "--locked", "--offline", "-p", "v1-game",
                    "--bin", "v1-game"], cwd=root, check=True)
    store = prepare_store(root, scratch)
    binary = root / "target/release/v1-game"
    wake_window_display()
    actions = {"route": route_script, "drops": drops_script, "costs": costs_script}[mode]()
    records = []
    for index in range(1, 3 if mode == "costs" else 2):
        steps = destination / f"{mode}-{index}-input.json"
        steps.write_text(json.dumps(actions, indent=2) + "\n")
        witness = destination / f"{mode}-{index}-record.json"
        witness.unlink(missing_ok=True)
        env = dict(environment, THRUM_SCENE="interface", THRUM_INTERFACE_TEXT="menu",
                   THRUM_FONT_STORE=str(store), THRUM_V1_FRAMES="360",
                   THRUM_UI_EVENT_SCRIPT=str(steps), THRUM_UI_EVENT_REPORT=str(witness))
        command = ["/usr/bin/caffeinate", "-d", "-i", "-u", str(binary)]
        if mode == "drops":
            command.extend(["--join-ticket", TICKET])
        before = load()
        with (destination / f"{mode}-{index}.log").open("wb") as log:
            subprocess.run(command, cwd=root, env=env, check=True, timeout=70,
                           stdout=log, stderr=subprocess.STDOUT)
        record = json.loads(witness.read_text())
        record["final"] = json.loads(record["final"])
        record.update(commit=revision, target="mac-metal", build="release", command=command,
                      binary_sha256=digest(binary), load_before=before, load_after=load())
        witness.write_text(json.dumps(record, indent=2) + "\n")
        if mode == "route":
            validate_route(record)
        elif mode == "drops":
            validate_drops(record)
        else:
            validate_costs(record["final"])
        records.append(witness.name)
    return {"commit": revision, "target": "mac-metal", "runs": records}
