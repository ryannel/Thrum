#!/usr/bin/env python3
"""Prove the V1 interface on the iPhone 18 Pro simulator, for function only."""

import argparse
import json
import math
import os
from pathlib import Path
import plistlib
import runpy
import shutil
import subprocess
import sys
import time
from urllib.parse import quote

import simulator_lifecycle as simulator
from capture_spool import decoded_frames
from capture_v1_simulator import load, validate_session, wait
from game_ui_status import prepare_store
from render2d import ROOT, digest, git
from t2_writer_probe import package

BUNDLE = "org.thrum.v1"
APP = ROOT / "target/v1-game-ios-sim/ThrumV1.app"
RUNNER = ROOT / "target/v1-game-ios-sim/uitest/ThrumV1UITests.xctestrun"
ORACLE = ROOT / "target/debug/examples/interface_status"
TICKET = "THRUM-ALPHA-29406-abc+def/%2F?note=#fragment-8432"


def require(value, message):
    if not value:
        raise ValueError(message)


def menu(state):
    return json.loads(state.get("interface_menu", "{}"))


def viewport(state):
    return json.loads(state.get("interface_viewport", "{}"))


def compare_status(raw, image, output, bounds, store):
    width, height = image["extent"]
    scale = image["interface_scale"]
    insets = output["insets"]
    require(len(bounds) == 21 and len({node for node, _ in bounds}) == 21,
            f"the status layout did not report all 21 nodes: {bounds}")
    for node, box in bounds:
        require(box is not None, f"status node {node} hidden")
        start, end = box
        for axis, length in enumerate((width, height)):
            require(insets[axis] <= start[axis] <= end[axis] <= length / scale - insets[axis + 2],
                    f"status node {node} {box} beyond UIKit's safe area {insets}")

    arguments = [image["extent"], scale, insets, image["tick"]]
    data = subprocess.check_output([ORACLE, store], input=(json.dumps(arguments) + "\n").encode())
    header, _, packed = data.partition(b"\n")
    region, size, mask_size = json.loads(header)
    expected, mask = packed[:size], packed[size:]
    [[left, top], [right, bottom]] = region
    require(len(mask) == mask_size and size == mask_size * 4, "incomplete reference")
    opaque = 0
    for y in range(top, bottom):
        for x in range(left, right):
            index = (y - top) * (right - left) + x - left
            at = (y * width + x) * 4
            opaque += mask[index] == 0
            for ch in range(4):
                actual, reference = raw[at + ch], expected[index * 4 + ch]
                if abs(actual - reference) > (mask[index] if ch < 3 else 0):
                    raise ValueError(f"status pixel {x},{y} channel {ch}: {actual} != {reference}")
    require(opaque > 0, "reference contained no exact pixels")
    return dict(oracle_sha256=digest(ORACLE), oracle_region=region, compared_pixels=mask_size,
                exact_pixels=opaque, one_code_blend_pixels=mask_size - opaque, status_bounds=bounds)


class Case:
    def __init__(self, out):
        self.out = out
        self.device = simulator.worker_device()
        self.sessions = 0
        self.report = dict(target="ios-sim", scope="function only", device=self.device,
                           commit=os.environ["THRUM_UI_SIM_COMMIT"], app_sha256=digest(APP / "ThrumV1"),
                           load=subprocess.check_output(["uptime"], text=True).strip(),
                           commands=[], images={}, tests={})
        self.simctl("install", self.device, str(APP))
        container = Path(self.simctl("get_app_container", self.device, BUNDLE, "data"))
        self.temp = container / "tmp"
        self.temp.mkdir(exist_ok=True)
        self.store = container / "Documents" / "interface-store"
        shutil.copytree(out / "store", self.store)
        self.report["font_head_sha256"] = digest(self.store / "HEAD")
        self.witness = self.temp / "thrum-v1-ios-state.txt"
        arguments = sys.argv
        sys.argv = ["route.py", self.device, str(self.witness)]
        os.environ["THRUM_V1_ROUTE_OUT"] = str(out / "route")
        try:
            self.route = runpy.run_path(str(ROOT / "examples/v1-game-ios/ios/route.py"))
        finally:
            sys.argv = arguments

    def simctl(self, *args, env=None):
        command = ["xcrun", "simctl", *args]
        result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, env=env, timeout=120)
        self.report["commands"].append(dict(command=command, exit=result.returncode,
                                            stdout=result.stdout, stderr=result.stderr))
        result.check_returncode()
        return result.stdout.strip()

    def state(self):
        error = self.temp / "thrum-capture-error.txt"
        if error.exists():
            raise RuntimeError(error.read_text())
        try:
            raw = self.witness.read_text()
        except FileNotFoundError:
            return {}
        if raw.startswith("fault="):
            raise RuntimeError(raw)
        return dict(line.split("=", 1) for line in raw.splitlines() if "=" in line)

    def expect(self, predicate, what, timeout=100):
        return wait(self.state, predicate, what, timeout)

    def screen(self, name):
        return self.expect(lambda s: menu(s).get("drawn") == name and
                           menu(s).get("drawn_disarmed") is False, f"submitted {name}")

    def reset_capture(self):
        for path in self.temp.glob("thrum-capture-*"):
            if path.is_dir():
                shutil.rmtree(path)
            else:
                path.unlink()
        self.witness.unlink(missing_ok=True)
        self.sessions = 0

    def launch(self, screen):
        self.reset_capture()
        child = dict(os.environ, SIMCTL_CHILD_THRUM_V1_ROUTE="1", SIMCTL_CHILD_THRUM_SCENE="interface",
                     SIMCTL_CHILD_THRUM_INTERFACE_TEXT=screen, SIMCTL_CHILD_THRUM_FONT_STORE=str(self.store))
        self.simctl("launch", self.device, BUNDLE, env=child)
        return self.expect(lambda s: int(s.get("presented", 0)) >= 10, f"presented {screen}")

    def post(self, command):
        self.route["post"](command)

    def test(self, name, *, arguments=None):
        tests = self.report["tests"]
        label = f"{name}-{len(tests) + 1}"
        config = RUNNER
        if arguments:
            # Keep __TESTROOT__ resolving to the actual test bundle.
            config = RUNNER.with_name("ThrumV1Interface.xctestrun")
            data = plistlib.loads(RUNNER.read_bytes())
            data["ThrumV1UITests"]["TestingEnvironmentVariables"].update(arguments)
            config.write_bytes(plistlib.dumps(data))
        command = ["xcodebuild", "test-without-building", "-xctestrun", str(config),
                   "-destination", f"id={self.device}", "-resultBundlePath", str(self.out / f"{label}.xcresult"),
                   f"-only-testing:ThrumV1UITests/ThrumV1Touch/{name}"]
        tests[label] = dict(arguments=arguments, command=command, log=f"{label}.log")
        with (self.out / tests[label]["log"]).open("w") as log:
            return subprocess.Popen(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT)

    def finish_test(self, process):
        try:
            require(process.wait(timeout=180) == 0, "XCTest failed: see the test log")
        finally:
            if process.poll() is None:
                process.terminate()

    def rotate(self, landscape):
        previous = self.state()
        self.finish_test(self.test("testLandscape" if landscape else "testPortrait"))

        def rotated(state):
            width, height = (int(part) for part in state.get("presented_extent", "0x0").split("x"))
            return width != height and (width > height) == landscape and (
                int(state.get("presented_epoch", 0)) > int(previous["presented_epoch"]))

        state = self.expect(rotated, "presented rotated drawable")
        require(all(state[k] == previous[k] for k in ("pid", "device_generation")),
                "rotation replaced the scene process or device")
        return state

    def grab(self, label, *, status=False):
        self.sessions += 1
        previous = self.state()
        self.post("hold-world")
        held = self.expect(lambda s: s.get("world_paused") == "true" and
                           int(s.get("pause_replies", 0)) > int(previous["pause_replies"]), "held world")
        self.post("capture-still")
        source = self.temp / f"thrum-capture-{self.sessions}"
        receipt = wait(lambda: load(source / "runtime.json"), bool, f"retired {label} still")
        validate_session(receipt)
        frame, = receipt["images"]
        identity = load(source / "identity.json")
        require(frame["tick"] == int(held["pause_tick"]) and
                frame["surface_epoch"] == int(held["presented_epoch"]) and
                frame["extent"] == [int(side) for side in held["presented_extent"].split("x")] and
                frame["device_generation"] == int(held["device_generation"]),
                f"capture does not match the held surface endpoint: {frame}")
        width, height = frame["extent"]
        require((width > height) == label.endswith("landscape"), f"{label} has the wrong orientation")
        screen = self.out / f"{label}-simctl.png"
        self.simctl("io", self.device, "screenshot", str(screen))
        area = identity["interface_output"]
        left, top, right, bottom = area["insets"]
        require(frame["interface_scale"] == area["density"] == 3 and identity["backend"] == "Metal",
                f"unexpected interface target or scale: {frame}, {area}")
        require((left > 0 and right > 0 if width > height else top > 0) and bottom > 0,
                f"UIKit safe area did not follow the orientation: {area}")

        engine = self.out / f"{label}-engine.png"
        raw, = decoded_frames(source / "frames.spool", identity["encoding"],
                              receipt["writer"]["hashes"], width * height * 4)
        package(raw, engine, width, height)
        identity_path = self.out / f"{label}-identity.json"
        receipt_path = self.out / f"{label}-capture.json"
        identity_path.write_text(json.dumps(identity, indent=2))
        receipt_path.write_text(json.dumps(receipt, indent=2))
        record = dict(mode="still", engine_png=engine.name, engine_sha256=digest(engine),
                      simctl_png=screen.name, simctl_sha256=digest(screen), held=held,
                      receipt=receipt_path.name, identity=identity_path.name, frame=frame)
        if status:
            record.update(compare_status(raw, frame, area, load(source / "status-bounds.json"), self.store))
        else:
            shown = viewport(held)
            require(shown == dict(extent=frame["extent"], scale=3, insets=area["insets"]),
                    f"submitted input grid does not match the drawable: {shown}, {area}")
        self.report["images"][label] = record
        shutil.rmtree(source)
        self.post("run-world")
        self.expect(lambda s: s.get("world_paused") == "false" and
                    int(s.get("world_tick", 0)) > int(held["pause_tick"]), "world resumed")

    def tap(self, button, state):
        box, = [box for box in menu(state)["boxes"] if box[0] == button]
        _, origin, size = box
        shown = viewport(state)
        point = [(origin[i] + size[i] / 2) * shown["scale"] / shown["extent"][i] for i in range(2)]
        require(all(side >= 44 for side in size), f"undersized control {box}")
        self.finish_test(self.test("testInterfaceTap", arguments={
            "THRUM_INTERFACE_TEST_POINT": f"{point[0]:.9f},{point[1]:.9f}",
        }))

    def open_url(self):
        url = "thrum-v1://join/" + quote(TICKET, safe="")
        process = self.state()["pid"]

        def delivered(state):
            report = menu(state)
            return (report.get("drawn") == "Joining" and report.get("ticket") == TICKET
                    and any(box[0] == 220 for box in report["boxes"])
                    and any(effect["effect"] == {"text": TICKET, "source": "Link"}
                            for effect in report["effects"]))

        self.simctl("openurl", self.device, url)
        observed = self.expect(lambda s: delivered(s) or s.get("state") == "Suspended",
                               "UIKit link delivery or confirmation")
        if not delivered(observed):
            self.simctl("io", self.device, "screenshot", str(self.out / "invite-confirmation.png"))
            self.finish_test(self.test("testConfirmInterfaceInvite"))
        state = self.expect(delivered, "UIKit URL delivered a complete ticket")
        require(state["pid"] == process, "opening the URL replaced the app")
        self.report["invite"] = dict(url=url, expected=TICKET, state=state)
        return self.screen("Joining")

    def layout(self):
        self.launch("status")
        # An XCTest rotation sets the simulator's device orientation even when
        # the previous simulator turn finished in another app.
        status = self.state()
        width, height = map(int, status["presented_extent"].split("x"))
        if width > height:
            self.rotate(False)
        self.grab("status-portrait", status=True)
        self.rotate(True)
        self.grab("status-landscape", status=True)
        self.simctl("terminate", self.device, BUNDLE)
        initial_menu = self.launch("menu")
        width, height = map(int, initial_menu["presented_extent"].split("x"))
        if width < height:
            self.rotate(True)
        self.post("open-choice")
        self.screen("Choice")
        self.grab("choice-landscape")
        self.rotate(False)
        before = self.screen("Choice")
        self.grab("choice-portrait")
        self.simctl("launch", self.device, "com.apple.Preferences")
        suspended = self.expect(lambda s: s.get("state") == "Suspended" and s.get("world_paused") == "true",
                                "background lifecycle")
        self.finish_test(self.test("testInterfaceForeground"))
        resumed = self.expect(lambda s: s.get("state") == "Active" and
                              menu(s).get("drawn") == "Choice" and
                              int(s.get("presented", 0)) > int(suspended["presented"]) and
                              int(s.get("world_tick", 0)) > int(suspended["world_tick"]),
                              "foreground choice and world")
        require(resumed["pid"] == before["pid"] and
                all(menu(before)[key] == menu(resumed)[key] for key in ("screen", "focus", "boxes", "drawn_cards"))
                and viewport(before) == viewport(resumed), "background discarded the choice layout or focus")
        self.report["lifecycle"] = dict(before=before, suspended=suspended, resumed=resumed)
        joined = self.open_url()
        width, height = viewport(joined)["extent"]
        if width > height:
            self.rotate(False)
        self.grab("join-portrait")
        self.rotate(True)
        self.screen("Joining")
        self.grab("join-landscape")

    def touch(self):
        self.launch("menu")
        before = self.screen("Play")
        if viewport(before)["extent"][0] > viewport(before)["extent"][1]:
            self.rotate(False)
            self.screen("Play")
        strokes = []
        for index, card in enumerate([110, 120, 130, 140]):
            baseline = self.state()
            self.tap(92, baseline)
            opened = self.screen("Choice")
            before, _ = self.route["sample"](f"card {card} before")
            self.tap(card, opened)
            after = self.screen("Play")
            position, _ = self.route["sample"](f"card {card} after")
            require(before == position and after["axes_moving"] == baseline["axes_moving"] and
                    after["touch_starts"] == baseline["touch_starts"] and
                    menu(after)["inputs"] == menu(baseline)["inputs"],
                    f"an interface-owned touch reached gameplay for card {card}")
            expected = [int(i <= index) for i in range(4)]
            require(menu(after)["activations"] == expected, f"unexpected card activations: {menu(after)}")
            strokes.append(dict(card=card, opened=opened, after=after, actor_before=before, actor_after=position))
        self.report["card_taps"] = strokes

        driver = self.test("testHold")
        try:
            baseline, initial = self.route["sample"]("initial position")
            moving = self.expect(lambda s: s.get("last_axes") == "0,1,0" and
                                 int(s.get("touch_starts", 0)) > int(initial["touch_starts"]),
                                 "a real touch deflects the virtual stick", 180)
            self.post("open-choice")
            interrupted = self.screen("Choice")
            require(interrupted["last_axes"] == "0,0,0", "modal did not interrupt the stick")
            stop, _ = self.route["sample"]("modal position")
            next_stop, _ = self.route["sample"]("later modal position")
            require(stop == next_stop and math.dist(baseline, stop) > 0, "actor did not stop inside the choice")
            self.post("close-choice")
            resumed = self.screen("Play")
            require(resumed["last_axes"] == "0,1,0" and resumed["touch_starts"] == moving["touch_starts"],
                    "held touch did not resume before a new touch was started")
            continuing, _ = self.route["sample"]("resumed position")
            require(math.dist(stop, continuing) > 0, "resumed stick did not move the actor")
            self.report["held_touch"] = dict(moving=moving, interrupted=interrupted, resumed=resumed,
                                            start=baseline, stopped=stop, after=continuing)
        finally:
            self.finish_test(driver)
        self.expect(lambda s: int(s.get("touch_releases", 0)) > int(moving["touch_releases"]) and
                    s.get("last_axes") == "0,0,0", "held contact released")
        self.open_url()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("case", choices=["layout", "touch"])
    parser.add_argument("--output", type=Path)
    parser.add_argument("--worker", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    os.chdir(ROOT)
    out = (args.output or ROOT / "tmp" / "game-ui-wp5" / f"{args.case}-{time.time_ns()}").resolve()
    if not args.worker:
        out.mkdir(parents=True)
        simulator.admission()
        os.environ["THRUM_UI_SIM_COMMIT"] = git("rev-parse", "HEAD")
        os.environ["RUSTC_WRAPPER"] = ""
        prepare_store(ROOT, out)
        if args.case == "layout":
            subprocess.run(["rustup", "run", "stable", "cargo", "build", "--locked", "--offline",
                            "-p", "v1-game", "--example", "interface_status"], check=True, timeout=600)
        for cmd in [["examples/v1-game-ios/ios/build-simulator.sh", "--proving"],
                    ["examples/v1-game-ios/ios/build-uitest.sh"]]:
            subprocess.run(cmd, check=True, timeout=600)
        require(os.environ["THRUM_UI_SIM_COMMIT"] == git("rev-parse", "HEAD"),
                "the source revision changed while the proof was building")
        os.environ[simulator.RECORD_ENV] = str(out / "simulator-lifecycle.jsonl")
        simulator.exec_with_admission(["/usr/bin/caffeinate", "-d", "-i", "-u",
                                       sys.executable, simulator.__file__, "fresh", "--",
                                       sys.executable, __file__, args.case, "--output", out, "--worker"])
    proof = Case(out)
    try:
        getattr(proof, args.case)()
        proof.report["complete"] = True
    finally:
        result = out / "result.json"
        result.write_text(json.dumps(proof.report, indent=2) + "\n")
        print(result, flush=True)


if __name__ == "__main__":
    main()
