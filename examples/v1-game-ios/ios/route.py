"""G3b iOS simulator route: walks the shell's function rows on one launched app.

Usage: route.py <device-udid> <state-file>. Called by check-simulator.sh after
it launches the app with THRUM_V1_ROUTE=1. Function evidence only: simulator
timing is never gate evidence, so thresholds here only separate a UIKit
callback that waited on the render owner from one that did not.
"""

from __future__ import annotations

import json
import hashlib
import math
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

DEVICE, STATE = sys.argv[1], Path(sys.argv[2])
OUT = Path(os.environ.get("THRUM_V1_ROUTE_OUT", "target/v1-game-ios-sim/route"))
TOUCH = os.environ.get("THRUM_V1_TOUCH", "xcuitest")
XCTESTRUN = Path(os.environ.get("THRUM_V1_XCTESTRUN", "target/v1-game-ios-sim/uitest/ThrumV1UITests.xctestrun"))
# xcodebuild can take most of a minute to start a test on a fresh device.
TOUCH_WINDOW = 180.0
# Simulator wall time is not evidence: host load can deschedule any callback.
# The gate is structural: while the render owner holds its lease section for
# 8 s, a UIKit callback that waited on it would take at least that long.
STALL_UIKIT_MAX_US = 1_000_000
# A one-second swipe at full deflection moves the actor about 6 m; a stale
# event must not move it measurably.
MIN_SWIPE_TRAVEL = 1.0
# After a resize, the owner waits for at most two pending frames before it
# reconfigures; six replies leaves room for the retiring frames.
MAX_RESIZE_GAP_REPLIES = 6
STRIP_FPS = 20
STILL_TOLERANCE = 1e-3
RENDER_THREAD = "thrum-v1-ios-render"
WORLD_THREAD = "thrum-world"

OUT.mkdir(parents=True, exist_ok=True)
log_lines: list[str] = []


def log(text: str) -> None:
    print(text, flush=True)
    log_lines.append(text)
    (OUT / "route.log").write_text("\n".join(log_lines) + "\n")


def read() -> dict[str, str]:
    try:
        raw = STATE.read_text()
    except FileNotFoundError:
        return {}
    if raw.startswith("fault="):
        fail(f"app fault: {raw.strip()}")
    return dict(line.split("=", 1) for line in raw.splitlines() if "=" in line)


def num(state: dict[str, str], key: str) -> int:
    return int(state.get(key, "0"))


def axes(state: dict[str, str]) -> list[float]:
    return [float(value) for value in state.get("last_axes", "0,0,0").split(",")]


def fail(reason: str, state: dict[str, str] | None = None) -> None:
    log(f"FAIL: {reason}")
    if state is not None:
        log(" ".join(f"{key}={value}" for key, value in state.items()))
    sys.exit(1)


def wait(predicate, timeout: float, what: str) -> dict[str, str]:
    deadline = time.monotonic() + timeout
    state: dict[str, str] = {}
    while time.monotonic() < deadline:
        state = read()
        if state and predicate(state):
            return state
        time.sleep(0.1)
    fail(f"timed out after {timeout:.0f} s waiting for {what}", state)
    raise AssertionError


def check(condition: bool, reason: str, state: dict[str, str]) -> None:
    if not condition:
        fail(reason, state)


def load() -> str:
    """Host load, logged beside rows: the simulator shares this Mac."""
    out = subprocess.run(["uptime"], capture_output=True, text=True).stdout
    return out.split("load averages:")[-1].strip()


def step(text: str) -> None:
    log(f"step {time.strftime('%H:%M:%S')}: {text}")


def simctl(*args: str, background: bool = False):
    command = ["xcrun", "simctl", *args]
    if args[0] != "io":
        step(" ".join(command))
    if background:
        return subprocess.Popen(command, stdout=subprocess.DEVNULL)
    subprocess.run(command, check=True, timeout=90, stdout=subprocess.DEVNULL)
    return None


COMMANDS = STATE.parent / "thrum-v1-commands"
command_sequence = 0


def post(command: str) -> None:
    """Send a qualification command by dropping a file into the app's container.

    The app watches the directory with a dispatch source on its main queue.
    `simctl spawn notifyutil` (the Darwin-notification path, still supported)
    can take over a minute on a loaded host; a file write cannot.
    """
    global command_sequence
    command_sequence += 1
    step(f"command {command} (file drop #{command_sequence})")
    COMMANDS.mkdir(parents=True, exist_ok=True)
    partial = STATE.parent / f".thrum-v1-command-{command_sequence}"
    partial.write_text(command)
    partial.rename(COMMANDS / f"{command_sequence:06d}-{command}")


def foreground() -> None:
    """Bring the running app back to the foreground (launch does not relaunch it)."""
    simctl("launch", DEVICE, "org.thrum.v1")


def sample(label: str) -> tuple[list[float], dict[str, str]]:
    """Read the committed controlled-actor position from the world lane's page.

    An explicit route witness: the shell scans the page only on this command,
    never on display callbacks or render reports.
    """
    before = num(read(), "actor_samples")
    post("sample-actor")
    s = wait(lambda s: num(s, "actor_samples") > before, 15, f"a committed actor sample ({label})")
    if s.get("actor_position", "none") == "none":
        fail(f"no committed controlled actor in the published page ({label})", s)
    position = [float(value) for value in s["actor_position"].split(",")]
    log(f"actor {label}: {', '.join(f'{v:.4f}' for v in position)}")
    return position, s


def extent_proof(state: dict[str, str], expected_fov: float = 65.0) -> dict:
    """Resource/matrix witness from an identified, actually presented token."""
    raw = state.get("frame_extent", "none")
    check(raw != "none", "missing encoded-frame extent witness", state)
    attachments, matrix, source, destination, viewport, submission, revision, textures, texture_bytes = raw.split(";")
    grid = lambda value: [int(side) for side in value.split("x")]
    surface = grid(state["presented_extent"])
    targets = [grid(value) for value in attachments.split(",")]
    drawable = grid(state.get("drawable", "0x0"))
    view = [int(value) for value in viewport.split(",")]
    check(all(target == surface for target in targets), "actual scene attachments differ from surface", state)
    check(grid(source) == grid(destination) == drawable == surface,
          "presenter source/destination or actual drawable differs from scene grid", state)
    check(view == [0, 0, *surface], "presenter does not use the full surface viewport", state)
    columns = [float(value) for value in matrix.split(",")]
    check(len(columns) == 16 and all(math.isfinite(value) for value in columns), "invalid encoded matrix", state)
    fx = math.sqrt(sum(columns[column * 4] ** 2 for column in range(3)))
    fy = math.sqrt(sum(columns[column * 4 + 1] ** 2 for column in range(3)))
    aspect = fy / fx
    fov = math.degrees(2 * math.atan(1 / fy))
    check(math.isclose(aspect, surface[0] / surface[1], rel_tol=1e-5), "encoded projection aspect differs from surface", state)
    check(math.isclose(fov, expected_fov, abs_tol=1e-3), "selected camera vertical field of view changed", state)
    return dict(attachments=targets, drawable=drawable, source=grid(source), destination=grid(destination),
                viewport=view, projection_columns=columns, aspect=aspect, vertical_fov_degrees=fov,
                submission=int(submission), revision=int(revision), epoch=num(state, "presented_epoch"),
                generation=num(state, "presented_generation"), presented=num(state, "presented"),
                textures_per_slot=int(textures), texture_bytes_per_slot=int(texture_bytes))


def snapshot(name: str, state: dict[str, str], screenshot: bool = True) -> None:
    (OUT / f"{name}.txt").write_text("".join(f"{k}={v}\n" for k, v in state.items()))
    if state.get("state") == "Active" and num(state, "presented_epoch") == num(state, "epoch"):
        (OUT / f"{name}-extent.json").write_text(json.dumps(extent_proof(state), indent=2) + "\n")
    if screenshot:
        before = read()
        began = time.monotonic()
        simctl("io", DEVICE, "screenshot", str(OUT / f"{name}.png"))
        after = read()
        if state.get("state") == "Active" and num(state, "presented_epoch") == num(state, "epoch"):
            first, last = extent_proof(before), extent_proof(after)
            check(first["epoch"] == last["epoch"] and first["drawable"] == last["drawable"],
                  "screenshot crossed an extent transition", after)
            (OUT / f"{name}-capture.json").write_text(json.dumps(dict(before=first, after=last,
                capture_wall_seconds=time.monotonic() - began,
                scope="simctl settled-state bracket, compositor frame identity unavailable"), indent=2) + "\n")


def presents_after(base: dict[str, str], frames: int = 60):
    return lambda s: s.get("state") == "Active" and num(s, "presented") >= num(base, "presented") + frames


def settled(state: dict[str, str]) -> bool:
    return num(state, "live_detached_windows") == 0


def startup() -> dict[str, str]:
    s = wait(lambda s: num(s, "presented") >= 600, 120, "600 presented frames")
    check(s.get("display_thread") == "main", "display targets are not forwarded on UIKit main", s)
    check(s.get("prepare_requested_on") == "main", "prepare was not requested from UIKit main", s)
    check(s.get("prepare_thread") == RENDER_THREAD, "device preparation ran off the render owner", s)
    check(s.get("frame_thread") == RENDER_THREAD, "frames ran off the render owner", s)
    check(s.get("world_thread") == WORLD_THREAD, "world wakes do not come from the world lane", s)
    check(num(s, "max_pending_frames") <= 2, "more than two submitted frames", s)
    check(num(s, "acquired") <= num(s, "presentable_encodes"), "a drawable was acquired without a presentable encode", s)
    check(num(s, "completed_gpu") >= 590 and num(s, "surface_samples") >= 590, "GPU completions lag presents", s)
    check(num(s, "display_link_callbacks") >= 600, "display link callbacks below 600", s)
    check("Apple_iOS_simulator_GPU" in s.get("adapter", ""), "simulator Metal adapter not selected", s)
    first = extent_proof(s)
    s = wait(presents_after(s, 120), 30, "steady presentation at the identical extent")
    second = extent_proof(s)
    check(first["revision"] == second["revision"], "steady identical extents rebuilt target resources", s)
    (OUT / "steady-extent.json").write_text(json.dumps(dict(before=first, after=second,
        target_ring_replacements=0, target_texture_creations=0), indent=2) + "\n")
    snapshot("01-startup", s)
    log(f"startup: presented={s['presented']} completed={s['completed_gpu']} "
        f"callbacks={s['display_link_callbacks']} pending<={s['max_pending_frames']} "
        f"threads display/prepare/frame/world={s['display_thread']}/{s['prepare_thread']}/"
        f"{s['frame_thread']}/{s['world_thread']}")
    return s


def start_stall(command: str = "stall-render") -> None:
    """Request an 8 s render-owner stall and return once the owner holds it.

    The revocation that follows takes a `simctl spawn` (over a second, longer
    on a loaded host) to arrive, well inside the held section. Each row still
    proves it landed inside by its deferred release.
    """
    stalls = num(read(), "stalls")
    post(command)
    wait(lambda s: s.get("stall_active") == "true" or num(s, "stalls") > stalls, 15, f"the {command} section to hold")
    check(read().get("stall_active") == "true", f"the {command} section ended before the revocation was sent", read())


def stall(base: dict[str, str]) -> dict[str, str]:
    post("stall-render")
    s = wait(lambda s: num(s, "stalls") > num(base, "stalls"), 15, "the render-owner stall to end")
    check(s.get("stall_status") == "Presented", "a current-epoch stall did not present", s)
    check(num(s, "stall_targets") - num(base, "stall_targets") >= 30, "display callbacks stopped during the stall", s)
    check(num(s, "stall_main_max_us") < STALL_UIKIT_MAX_US, "UIKit waited on the stalled render owner", s)
    ticks = num(s, "stall_tick_end") - num(s, "stall_tick_start")
    check(ticks >= 30, "world ticks did not advance during the stall", s)
    s = wait(presents_after(s), 15, "presentation after the stall")
    snapshot("02-stall", s, screenshot=False)
    log(f"stall: 8 s inside the lease section; callbacks during stall={num(s, 'stall_targets') - num(base, 'stall_targets')}, "
        f"world ticks during stall={ticks}, UIKit max during stall={s['stall_main_max_us']} us")
    return s


def hide(base: dict[str, str]) -> dict[str, str]:
    post("hide-view")
    h1 = wait(lambda s: num(s, "denied") >= num(base, "denied") + 120, 15, "denied drawables")
    time.sleep(2)
    h2 = wait(lambda s: num(s, "denied") >= num(h1, "denied") + 120, 15, "more denied drawables")
    check(num(h2, "acquired") == num(h1, "acquired"), "a drawable was acquired while hidden", h2)
    check(num(h2, "presented") == num(h1, "presented"), "a frame presented while hidden", h2)
    check(num(h2, "world_tick") > num(h1, "world_tick"), "world ticks stopped while hidden", h2)
    check(num(h2, "missing") == num(base, "missing"), "hidden frames were counted as missing", h2)
    snapshot("03-hidden", h2)
    post("show-view")
    s = wait(presents_after(h2), 15, "presentation after showing the view")
    log(f"denied: {num(h2, 'denied') - num(h1, 'denied')} denied, 0 acquired, 0 presented, "
        f"{num(h2, 'world_tick') - num(h1, 'world_tick')} world ticks while hidden; missing stays {h2['missing']}")
    return s


def replace(base: dict[str, str]) -> dict[str, str]:
    post("replace-surface")
    s = wait(lambda s: num(s, "replacements") > num(base, "replacements")
             and num(s, "presented_epoch") > num(base, "epoch") and presents_after(base)(s) and settled(s),
             20, "presentation on a replacement surface")
    check(s["pid"] == base["pid"], "the process changed", s)
    check(num(s, "installs") == num(base, "installs") + 1, "replacement did not install one new surface", s)
    check(num(s, "detached_windows") == num(base, "detached_windows") + 1, "the old window was not detached", s)
    check(num(s, "world_tick") > num(base, "world_tick"), "world ticks stopped across replacement", s)
    snapshot("04-replaced", s)
    log(f"replacement: epoch {base['epoch']} -> {s['epoch']}, presented on epoch {s['presented_epoch']}, "
        f"same pid {s['pid']}, live detached windows {s['live_detached_windows']}")
    return s


def stalled_replace(base: dict[str, str]) -> dict[str, str]:
    start_stall()
    post("replace-surface")
    s = wait(lambda s: num(s, "replacements") > num(base, "replacements")
             and num(s, "release_completed") > num(base, "release_completed")
             and num(s, "presented_epoch") > num(base, "epoch") and presents_after(base)(s) and settled(s),
             30, "deferred release and recovery after a stalled replacement")
    check(num(s, "release_deferred") > num(base, "release_deferred"), "the held section did not defer release", s)
    check(num(s, "stale_dropped") > num(base, "stale_dropped"), "the revoked drawable was not dropped", s)
    check(s.get("stall_status") == "StaleDropped", "the stalled frame was not rejected as stale", s)
    check(num(s, "raced_presents") == num(base, "raced_presents"), "a present raced revocation", s)
    snapshot("05-stalled-replace", s, screenshot=False)
    log(f"stalled replacement: deferred={s['release_deferred']} stale_dropped={s['stale_dropped']} "
        f"completed releases={s['release_completed']}; presents resumed on epoch {s['presented_epoch']}")
    return s


def raced_present(base: dict[str, str]) -> dict[str, str]:
    """Revoke while an admitted section holds an acquired drawable before present.

    The section passed its last epoch check, so it may present once onto the
    old layer, which stays attached until the section releases the lease. The
    detach, and the next present, wait for that release; UIKit never waits.
    """
    start_stall("stall-present")
    post("replace-surface")
    s = wait(lambda s: num(s, "replacements") > num(base, "replacements")
             and num(s, "release_completed") > num(base, "release_completed")
             and num(s, "presented_epoch") > num(base, "epoch") and presents_after(base)(s) and settled(s),
             30, "deferred release and recovery after a revocation inside the present window")
    check(num(s, "release_deferred") > num(base, "release_deferred"), "the held section did not defer release", s)
    check(num(s, "raced_presents") == num(base, "raced_presents") + 1,
          "the admitted section did not finish its one present after revocation", s)
    check(s.get("stall_status") == "Presented", "the admitted section did not present", s)
    check(num(s, "stall_main_max_us") < STALL_UIKIT_MAX_US, "UIKit waited on the held present", s)
    check(num(s, "detached_windows") == num(base, "detached_windows") + 1, "the old window was not detached", s)
    snapshot("05b-raced-present", s, screenshot=False)
    log(f"raced present: one admitted present after revocation (raced_presents={s['raced_presents']}), "
        f"deferred={s['release_deferred']}, old window detached after release; presents resumed on epoch "
        f"{s['presented_epoch']}")
    return s


def background(base: dict[str, str], name: str, during_stall: bool = False) -> dict[str, str]:
    if during_stall:
        start_stall()
    simctl("launch", DEVICE, "com.apple.Preferences")
    b = wait(lambda s: s.get("state") == "Suspended" and num(s, "pause_replies") > num(base, "pause_replies"),
             20, "suspend and a confirmed world pause")
    check(b.get("world_paused") == "true", "the world pause was not confirmed", b)
    check(b.get("lifecycle_thread") == "main", "lifecycle ran off UIKit main", b)
    check(num(b, "lifecycle_main_max_us") < 1_000_000, "a lifecycle callback blocked", b)
    time.sleep(2)
    snapshot(f"{name}-background", b)
    foreground()
    s = wait(lambda s: num(s, "resume_replies") > num(base, "resume_replies") and presents_after(b)(s) and settled(s),
             20, "foreground resume and presentation")
    check(s["pid"] == base["pid"], "the process changed across background", s)
    check(s["resume_epoch"] == s["pause_epoch"], "the world identity changed across background", s)
    check(s["resume_tick"] == s["pause_tick"], "world ticks advanced while backgrounded", s)
    check(num(s, "presented_epoch") > num(base, "epoch"), "presentation did not resume on a new epoch", s)
    check(num(s, "world_tick") > num(s, "resume_tick"), "world ticks did not resume", s)
    if during_stall:
        check(num(s, "release_deferred") > num(base, "release_deferred"), "background did not defer the held section", s)
        check(num(s, "stale_dropped") > num(base, "stale_dropped"), "the stalled drawable was not dropped", s)
    snapshot(f"{name}-resumed", s)
    log(f"{name}: pid {s['pid']}, world epoch {s['pause_epoch']} at tick {s['pause_tick']} paused and resumed "
        f"at tick {s['resume_tick']}; surface epoch {base['epoch']} -> {s['epoch']}; "
        f"deferred releases {s['release_deferred']}")
    return s


def gesture(test: str) -> subprocess.Popen:
    """Start one real HID gesture from the XCUITest runner on the running app."""
    step(f"xcodebuild test-without-building -only-testing:ThrumV1UITests/ThrumV1Touch/{test} (HID gesture)")
    output = (OUT / f"xcuitest-{test}.log").open("a")
    return subprocess.Popen(
        ["xcodebuild", "test-without-building", "-xctestrun", str(XCTESTRUN),
         "-destination", f"id={DEVICE}", f"-only-testing:ThrumV1UITests/ThrumV1Touch/{test}"],
        stdout=output, stderr=subprocess.STDOUT)


def touch_window(name: str, test: str, predicate, what: str) -> tuple[dict[str, str], subprocess.Popen]:
    log(f"touch {name}: XCUITest {test}")
    driver = gesture(test)
    return wait(predicate, TOUCH_WINDOW, what), driver


def finish(driver: subprocess.Popen) -> None:
    # A gesture interrupted by background or replacement may report failure;
    # the app's witnesses, not the runner, decide each row.
    driver.wait(timeout=120)


def at_rest(state: dict[str, str]) -> bool:
    return all(value == 0.0 for value in axes(state))


def distance(a: list[float], b: list[float]) -> float:
    return math.dist(a, b)


def crop(name: str) -> None:
    """Zoom on the band right of the grid's top-right corner (iPhone 18 Pro pixels)."""
    source, target = OUT / f"{name}.png", OUT / f"{name}-corner.png"
    subprocess.run(["sips", "-c", "160", "420", "--cropOffset", "880", "780", str(source), "--out", str(target)],
                   check=True, stdout=subprocess.DEVNULL)
    subprocess.run(["sips", "-z", "480", "1260", str(target)], check=True, stdout=subprocess.DEVNULL)


def nudge(base: dict[str, str]) -> dict[str, str]:
    """Step the controlled actor just past the grid edge, where it is visible alone."""
    p0, n0 = sample("before nudge")
    snapshot("08a-before-nudge", n0)
    crop("08a-before-nudge")
    s, driver = touch_window("nudge", "testNudge",
                             lambda s: num(s, "touch_releases") > num(n0, "touch_releases")
                             and num(s, "axes_moving") > num(n0, "axes_moving") and at_rest(s),
                             "a nudge that moves and releases")
    finish(driver)
    time.sleep(0.5)
    p1, s = sample("after nudge")
    check(0.1 <= p1[0] - p0[0] <= 1.5 and abs(p1[1] - p0[1]) <= STILL_TOLERANCE,
          "the nudge did not step the committed actor sideways", s)
    snapshot("08a-after-nudge", s)
    crop("08a-after-nudge")
    log(f"nudge: committed actor moved {p1[0] - p0[0]:.3f} m right, past the grid edge "
        f"(compare 08a-before-nudge-corner.png with 08a-after-nudge-corner.png)")
    return s


def touches(base: dict[str, str]) -> dict[str, str]:
    base = nudge(base)
    p0, t0 = sample("before swipe")
    snapshot("08-before-swipe", t0)
    s, driver = touch_window("swipe", "testSwipe",
                     lambda s: num(s, "touch_releases") > num(t0, "touch_releases")
                     and num(s, "axes_moving") > num(t0, "axes_moving") and at_rest(s),
                     "a swipe that moves and releases")
    finish(driver)
    check(s.get("touch_thread") == "main", "touches were not forwarded on UIKit main", s)
    time.sleep(0.5)
    p1, s = sample("after swipe")
    check(distance(p0, p1) >= MIN_SWIPE_TRAVEL, "the swipe did not move the committed actor", s)
    snapshot("08-touch-swipe", s)
    log(f"touch: starts={s['touch_starts']} moving sends={s['axes_moving']} released to rest; "
        f"committed actor moved {distance(p0, p1):.3f} m")

    t1 = s
    s, driver = touch_window("hold-background", "testHold",
                     lambda s: num(s, "axes_moving") > num(t1, "axes_moving") and not at_rest(s),
                     "a held moving touch")
    simctl("launch", DEVICE, "com.apple.Preferences")
    b = wait(lambda s: s.get("state") == "Suspended" and at_rest(s), 20, "background clearing held input")
    check(num(b, "touch_cancels") + num(b, "touch_clears") > num(t1, "touch_cancels") + num(t1, "touch_clears"),
          "background did not cancel or clear the held touch", b)
    finish(driver)
    foreground()
    s = wait(presents_after(b), 20, "presentation after the held-touch background")
    check(at_rest(s), "held input survived background", s)
    log(f"background while held: cancels={s['touch_cancels']} clears={s['touch_clears']}, axes at rest")

    t2 = s
    s, driver = touch_window("hold-replace", "testHold",
                     lambda s: num(s, "axes_moving") > num(t2, "axes_moving") and not at_rest(s),
                     "a held moving touch")
    post("replace-surface")
    r = wait(lambda s: num(s, "replacements") > num(t2, "replacements") and at_rest(s), 20,
             "replacement clearing held input")
    check(num(r, "touch_releases") == num(t2, "touch_releases"),
          "the held finger released before the replacement landed (route command too slow)", r)
    time.sleep(0.5)
    p_clear, sampled = sample("after replacement cleared held input")
    # Hiding the detached window cancels its touch; that and any later event
    # for the old touch arrive under the old window id and count stale.
    # UIKit may deliver the old touch's events (one or two) before or after
    # the first sample. Either way they arrive after the clear: the row needs
    # no moving send since the clear and an unchanged committed position.
    s = wait(lambda s: num(s, "touch_stale") > num(t2, "touch_stale")
             and num(s, "old_window_touches") > num(t2, "old_window_touches"), TOUCH_WINDOW,
             "the old touch's late events counted stale")
    bracketed = num(s, "old_window_touches") > num(sampled, "old_window_touches")
    finish(driver)
    time.sleep(1)
    s = read()
    check(at_rest(s), "a stale release changed held input", s)
    check(num(s, "axes_moving") == num(r, "axes_moving"), "a stale event sent moving axes", s)
    p_stale, s = sample("after the old window's late events")
    check(distance(p_clear, p_stale) <= STILL_TOLERANCE, "a stale old-window event moved the committed actor", s)
    snapshot("09-stale-release", s)
    log(f"replacement while held: clears={s['touch_clears']} stale={s['touch_stale']} "
        f"old-window events={s['old_window_touches']}, axes stayed at rest; committed actor moved "
        f"{distance(p_clear, p_stale):.6f} m between samples ({'events between samples' if bracketed else 'events before the first sample'})")

    t3 = s
    s, driver = touch_window("swipe-after", "testSwipe",
                     lambda s: num(s, "touch_releases") > num(t3, "touch_releases")
                     and num(s, "axes_moving") > num(t3, "axes_moving") and at_rest(s),
                     "a swipe after recovery")
    finish(driver)
    time.sleep(0.5)
    p4, s = sample("after the recovered swipe")
    check(distance(p_stale, p4) >= MIN_SWIPE_TRAVEL, "the recovered swipe did not move the committed actor", s)
    snapshot("10-touch-after-recovery", s)
    log(f"touch after recovery: moving sends={s['axes_moving']}; committed actor moved {distance(p_stale, p4):.3f} m")
    return s


def extent(state: dict[str, str]) -> tuple[int, int]:
    width, height = state.get("presented_extent", "0x0").split("x")
    return int(width), int(height)


def record(name: str) -> tuple[subprocess.Popen, float]:
    """Record the simulator's framebuffer (an offscreen capture of the running game)."""
    path = OUT / f"{name}.mp4"
    step(f"xcrun simctl io {DEVICE} recordVideo {path.name}")
    started = time.monotonic()
    process = subprocess.Popen(["xcrun", "simctl", "io", DEVICE, "recordVideo", "--codec=h264", "--force", str(path)],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1.5)
    return process, started


def strip(name: str, recording: subprocess.Popen, started: float, resized_at: float) -> None:
    """Stop the recording and tile STRIP_FPS frames from 1 s before to 2 s after the resize."""
    recording.send_signal(signal.SIGINT)
    recording.wait(timeout=30)
    video, frames = OUT / f"{name}.mp4", OUT / f"{name}-frames"
    frames.mkdir(exist_ok=True)
    begin = max(0.0, resized_at - started - 1.0)
    # The capture keeps the device's native canvas; its display-matrix rotation
    # would change tile sizes mid-strip (and leave an EXIF orientation in the
    # PNGs), so frames stay unrotated.
    source = ["-display_rotation", "0", "-ss", f"{begin:.2f}", "-t", "3", "-i", str(video)]
    subprocess.run(["ffmpeg", "-loglevel", "error", "-y", *source,
                    "-vf", f"fps={STRIP_FPS},scale=-2:240", str(frames / "%03d.png")], check=True)
    count = len(list(frames.glob("*.png")))
    check(count >= 3 * STRIP_FPS - 2, f"the {name} strip has {count} frames, under {STRIP_FPS} fps", read())
    subprocess.run(["ffmpeg", "-loglevel", "error", "-y", *source,
                    "-vf", f"fps={STRIP_FPS},scale=-2:360,tile=15x4", "-frames:v", "1", str(OUT / f"{name}-strip.png")],
                   check=True)
    (OUT / f"{name}-strip.json").write_text(json.dumps(dict(fps=STRIP_FPS, frames=count,
        video_seek_seconds=begin, resize_observed_since_record_command=resized_at-started,
        scope="20 fps simctl compositor samples, command/observation alignment only, no engine frame IDs"), indent=2) + "\n")
    log(f"{name}: {count} frames at {STRIP_FPS} fps from {begin:.2f} s ({name}-strip.png, {name}-frames/)")


def rotate(name: str, test: str, base: dict[str, str], landscape: bool) -> dict[str, str]:
    """One real orientation change: a resize on the same window, lease and world."""
    p0, b = sample(f"before {name}")
    # Only orientation changes resize: installs (startup, replacement, resume)
    # configure once at the layer's extent and raise no `Resized` change.
    check(num(b, "resize_events") == (0 if landscape else num(base, "resize_events")),
          "a window install or lifecycle row changed the extent", b)
    recording, started = record(name)
    driver = gesture(test)
    s = wait(lambda s: num(s, "resize_events") > num(b, "resize_events"), TOUCH_WINDOW, f"the {name} resize event")
    resized_at = time.monotonic()
    want = (lambda w, h: w > h) if landscape else (lambda w, h: h > w)
    s = wait(lambda s: want(*extent(s)) and presents_after(b, 120)(s), 20, f"presentation at the {name} extent")
    finish(driver)
    time.sleep(1.0)
    strip(name, recording, started, resized_at)
    p1, s = sample(f"after {name}")
    snapshot(name, s)
    check(want(*extent(s)), f"{name} did not present at the rotated extent", s)
    check(num(s, "epoch") > num(b, "epoch"), f"{name} did not bump the surface epoch", s)
    check(num(s, "installs") == num(b, "installs") and num(s, "replacements") == num(b, "replacements")
          and s.get("pid") == b.get("pid"), f"{name} replaced the window or process", s)
    # The designed race bound (as for revocation): one section admitted before
    # `Resized` may present once after it; UIKit never waits to prevent it.
    raced = num(s, "resize_raced_presents") - num(b, "resize_raced_presents")
    old_extent = num(s, "old_extent_presents") - num(b, "old_extent_presents")
    check(raced <= 1, f"{name} presented more than once on a resize-revoked epoch", s)
    check(old_extent <= raced,
          f"{name} presented an old-extent drawable outside the one raced section", s)
    check(num(s, "max_resize_gap_replies") <= MAX_RESIZE_GAP_REPLIES,
          f"{name} took more than {MAX_RESIZE_GAP_REPLIES} replies to present at the new extent", s)
    check(s.get("device_generation") == b.get("device_generation"), f"{name} rebuilt the device", s)
    extent_proof(s)
    if b.get("cooked_qualification_held") == "true":
        check(num(s, "world_tick") == num(b, "world_tick"), f"held qualification world advanced across {name}", s)
    else:
        check(num(s, "world_tick") > num(b, "world_tick"), f"world ticks stopped across {name}", s)
    check(distance(p0, p1) <= STILL_TOLERANCE, f"{name} moved the committed actor", s)
    log(f"{name}: host load {load()}")
    log(f"{name}: extent {b.get('presented_extent')} -> {s['presented_extent']}, epoch {b['epoch']} -> {s['epoch']}, "
        f"gap {s['max_resize_gap_replies']} replies (max so far), reconfigure waits "
        f"{num(s, 'reconfigure_waits') - num(b, 'reconfigure_waits')}, stale drops "
        f"{num(s, 'stale_dropped') - num(b, 'stale_dropped')} (extent {num(s, 'extent_drops') - num(b, 'extent_drops')}), "
        f"raced {raced}, old-extent presents {old_extent}; ticks {b['world_tick']} -> {s['world_tick']}; "
        f"committed actor moved {distance(p0, p1):.6f} m")
    return s


def rotation(base: dict[str, str]) -> dict[str, str]:
    s = rotate("12-landscape", "testLandscape", base, landscape=True)
    p0, t0 = sample("before landscape swipe")
    s, driver = touch_window("landscape-swipe", "testSwipe",
                             lambda s: num(s, "touch_releases") > num(t0, "touch_releases")
                             and num(s, "axes_moving") > num(t0, "axes_moving") and at_rest(s),
                             "a landscape swipe that moves and releases")
    finish(driver)
    time.sleep(0.5)
    p1, s = sample("after landscape swipe")
    check(distance(p0, p1) >= MIN_SWIPE_TRAVEL, "the landscape swipe did not move the committed actor", s)
    log(f"landscape swipe: committed actor moved {distance(p0, p1):.3f} m")
    return rotate("13-portrait", "testPortrait", s, landscape=False)



def recovered_after(base: dict[str, str], recoveries: int = 1):
    """One more publication, then presents on the new device generation."""
    return lambda s: (num(s, "recoveries") >= num(base, "recoveries") + recoveries
                      and num(s, "presented_generation") == num(s, "device_generation")
                      and num(s, "device_generation") > num(base, "device_generation")
                      and presents_after(base, 60)(s))


def check_recovery(name: str, base: dict[str, str], s: dict[str, str], p0: list[float], p1: list[float]) -> None:
    check(num(s, "losses") > num(base, "losses"), f"{name}: wgpu's lost callback was not observed", s)
    check(num(s, "recoveries") == num(base, "recoveries") + 1, f"{name}: not exactly one publication", s)
    check(num(s, "recover_failed") == num(base, "recover_failed"), f"{name}: a recovery attempt failed", s)
    check(num(s, "epoch") > num(base, "epoch"), f"{name}: the loss did not revoke the surface lease", s)
    # A held drawable's loss replaces the surface (walk step 6); a background
    # round trip releases and reinstalls it by design (M3).
    replaced = sum(num(s, key) - num(base, key) for key in ("loss_replacements", "resumes"))
    check(s.get("pid") == base.get("pid") and num(s, "installs") == num(base, "installs") + replaced,
          f"{name}: recovery replaced the window or process", s)
    check(num(s, "world_tick") > num(base, "world_tick"), f"{name}: world ticks stopped across recovery", s)
    check(distance(p0, p1) <= STILL_TOLERANCE, f"{name}: recovery moved the committed actor", s)
    check(num(s, "stall_main_max_us") < STALL_UIKIT_MAX_US, f"{name}: UIKit waited on the recovering owner", s)


def device_loss(base: dict[str, str]) -> dict[str, str]:
    """M6: a destroyed device recovers on the same window, lease and world."""
    p0, b = sample("before device loss")
    post("lose-device")
    s = wait(recovered_after(b), 120, "device loss, recovery and presents on the new generation")
    p1, s = sample("after device recovery")
    check_recovery("device loss", b, s, p0, p1)
    check(num(s, "loss_replacements") == num(b, "loss_replacements"),
          "a loss with no drawable held replaced the surface", s)
    snapshot("14-device-loss", s)
    log(f"device loss: generation {b['device_generation']} -> {s['device_generation']} "
        f"(lost {s['lost_generation']}), lost frames {num(s, 'lost_frames') - num(b, 'lost_frames')}, "
        f"stale-generation refusals {num(s, 'stale_generation_refusals') - num(b, 'stale_generation_refusals')}, "
        f"recovery {s['recover_wall_us']} us on the owner, epoch {b['epoch']} -> {s['epoch']}, presents on "
        f"generation {s['presented_generation']} at {s['presented_extent']}; ticks {b['world_tick']} -> "
        f"{s['world_tick']}; committed actor moved {distance(p0, p1):.6f} m")

    b = read()
    post("stale-wake")
    s = wait(lambda s: num(s, "stale_loss_wakes") > num(b, "stale_loss_wakes")
             and num(s, "stale_generation_refusals") > num(b, "stale_generation_refusals")
             and presents_after(b, 30)(s), 30, "a stale wake and a stale-generation request refused")
    check(num(s, "losses") == num(b, "losses") and s["device_generation"] == b["device_generation"]
          and s["presented_generation"] == b["device_generation"],
          "an old-generation wake or request reached the live device", s)
    log(f"stale generation: wake for generation {num(b, 'device_generation') - 1} ignored, a request carrying it "
        f"refused unrun; generation {s['device_generation']} keeps presenting")

    p0, b = sample("before held-frame loss")
    start_stall("stall-render")
    post("lose-device")
    s = wait(lambda s: num(s, "stalls") > num(b, "stalls") and recovered_after(b)(s), 120,
             "a loss inside a held section, then recovery")
    p1, s = sample("after held-frame recovery")
    check_recovery("held-frame loss", b, s, p0, p1)
    check(s.get("stall_status") == "DeviceLost", "the held drawable was not dropped as lost", s)
    check(num(s, "lost_held_drops") > num(b, "lost_held_drops"), "no held drawable was dropped on the lost device", s)
    check(num(s, "raced_presents") == num(b, "raced_presents"), "a present raced the loss", s)
    check(num(s, "lost_abandons") > num(b, "lost_abandons"), "the held frame was not abandoned on the lost device", s)
    check(num(s, "lost_empty_submits") == 0, "a frame on a lost device flushed an empty queue submit", s)
    check(num(s, "loss_replacements") == num(b, "loss_replacements") + 1 and settled(s),
          "the surface that held the lost drawable was not replaced", s)
    snapshot("15-held-loss", s)
    log(f"held-frame loss: the drawable held across the loss dropped as DeviceLost and its surface was replaced "
        f"(wgpu cannot discard a destroyed device's drawable); generation "
        f"{b['device_generation']} -> {s['device_generation']}, abandoned frames "
        f"{num(s, 'lost_abandons') - num(b, 'lost_abandons')}, empty submits on lost frames {s['lost_empty_submits']} "
        f"(healthy empty submits {s['empty_submits']}), stale loss wakes {s['stale_loss_wakes']}, "
        f"stale-generation refusals {s['stale_generation_refusals']}")

    p0, b = sample("before loss during rotation")
    post("hold-recover")
    post("lose-device")
    wait(lambda s: num(s, "recover_held") > num(b, "recover_held"), 60, "the recovery held after the loss")
    driver = gesture("testLandscape")
    h = wait(lambda s: num(s, "resize_events") > num(b, "resize_events"), TOUCH_WINDOW, "a rotation during recovery")
    check(num(h, "recoveries") == num(b, "recoveries"), "the recovery hold expired before the rotation landed", h)
    post("release-recover")
    s = wait(lambda s: recovered_after(b)(s) and extent(s)[0] > extent(s)[1], 120,
             "one recovery fitted to the rotated extent")
    finish(driver)
    p1, s = sample("after loss during rotation")
    check_recovery("loss during rotation", b, s, p0, p1)
    check(num(s, "recover_refitted") > num(b, "recover_refitted"), "the candidate was not fitted to the new extent", s)
    snapshot("16-loss-rotation", s)
    log(f"loss during rotation: one publication refitted to {s['recovered_extent']}, presents at {s['presented_extent']}")
    r = read()
    driver = gesture("testPortrait")
    s = wait(lambda s: extent(s)[1] > extent(s)[0] and presents_after(r, 60)(s), TOUCH_WINDOW, "portrait again")
    finish(driver)

    p0, b = sample("before loss across background")
    post("hold-recover")
    post("lose-device")
    wait(lambda s: num(s, "recover_held") > num(b, "recover_held"), 60, "the recovery held after the loss")
    simctl("launch", DEVICE, "com.apple.Preferences")
    wait(lambda s: s.get("state") == "Suspended", 30, "suspend during recovery")
    post("release-recover")
    d = wait(lambda s: num(s, "recover_deferred") > num(b, "recover_deferred"), 60, "the recovery deferred in background")
    check(num(d, "recoveries") == num(b, "recoveries"), "a recovery published in the background", d)
    foreground()
    s = wait(recovered_after(b), 120, "recovery after foreground")
    p1, s = sample("after loss across background")
    check_recovery("loss across background", b, s, p0, p1)
    snapshot("17-loss-background", s)
    log(f"loss across background: deferred {num(s, 'recover_deferred') - num(b, 'recover_deferred')} while "
        f"suspended, one publication after resume; generation {s['device_generation']}")

    p0, t0 = sample("before swipe after device recovery")
    s, driver = touch_window("swipe-after-device", "testSwipe",
                             lambda s: num(s, "touch_releases") > num(t0, "touch_releases")
                             and num(s, "axes_moving") > num(t0, "axes_moving") and at_rest(s),
                             "a swipe after device recovery")
    finish(driver)
    time.sleep(0.5)
    p1, s = sample("after swipe on the recovered device")
    check(distance(p0, p1) >= MIN_SWIPE_TRAVEL, "the swipe after recovery did not move the committed actor", s)
    log(f"swipe after device recovery: committed actor moved {distance(p0, p1):.3f} m")
    check(num(s, "lost_empty_submits") == 0, "a frame on a lost device flushed an empty queue submit", s)
    log(f"device rows: {s['lost_abandons']} frames abandoned on lost devices with no queue call, "
        f"{s['lost_empty_submits']} empty submits on lost frames, {s['empty_submits']} healthy empty submits")
    return s


def identity() -> None:
    executable = Path(os.environ.get("CARGO_TARGET_DIR", "target")) / "v1-game-ios-sim/ThrumV1.app/ThrumV1"
    paths = subprocess.check_output(["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"]).decode().split("\0")
    sources = {path: hashlib.sha256(Path(path).read_bytes()).hexdigest()
               for path in sorted(paths) if path and Path(path).is_file()
               and Path(path).suffix in (".rs", ".wgsl", ".py", ".swift", ".sh", ".toml", ".lock")}
    (OUT / "source-files.json").write_text(json.dumps(sources, indent=2) + "\n")
    record = dict(source_sha256=hashlib.sha256(json.dumps(sources, sort_keys=True).encode()).hexdigest(), commit=subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
                  patch_sha256=hashlib.sha256(subprocess.check_output(["git", "diff", "HEAD", "--binary"])).hexdigest(),
                  binary_sha256=hashlib.sha256(executable.read_bytes()).hexdigest(),
                  simulator=DEVICE, xcode=subprocess.check_output(["xcodebuild", "-version"], text=True).strip())
    (OUT / "identity.json").write_text(json.dumps(record, indent=2) + "\n")
    listing = json.loads(subprocess.run(["xcrun", "simctl", "list", "devices", "-j"], check=True,
                                        capture_output=True, text=True).stdout)
    for runtime, devices in listing["devices"].items():
        for device in devices:
            if device["udid"] == DEVICE:
                log(f"simulator: {device['name']} {DEVICE} type={device.get('deviceTypeIdentifier')} "
                    f"runtime={runtime} state={device['state']}")
                return
    fail(f"simulator {DEVICE} is not listed")


def main() -> None:
    log(f"G3b iOS route on {DEVICE} (function evidence only); host load {load()}")
    identity()
    log("manual steps: none; every interaction below is a logged simctl command, Darwin notification "
        "or XCUITest HID gesture")
    s = startup()
    rows = os.environ.get("THRUM_V1_ROWS")
    if rows in ("rotation", "device"):
        # Development only: the named route runs every row.
        rotation(s) if rows == "rotation" else device_loss(s)
        log(f"PARTIAL: {rows} rows only (THRUM_V1_ROWS={rows})")
        sys.exit(2)
    s = stall(s)
    s = hide(s)
    s = replace(s)
    s = stalled_replace(s)
    s = raced_present(s)
    s = background(s, "06")
    s = background(s, "07-stall", during_stall=True)
    if TOUCH == "skip":
        log("PARTIAL: touch rows skipped (THRUM_V1_TOUCH=skip)")
        sys.exit(2)
    s = touches(s)
    s = rotation(s)
    s = device_loss(s)
    check(num(s, "max_pending_frames") <= 2, "more than two submitted frames", s)
    log(f"UIKit handler maxima: frame/input {s['main_max_us']} us ({s.get('main_max_kind')}), "
        f"lifecycle {s['lifecycle_main_max_us']} us ({s.get('lifecycle_main_max_kind')}), "
        f"during stalls {s['stall_main_max_us']} us; world overloads {s['overloads']} ({s['refused_ticks']} refused ticks)")
    check(num(s, "stall_main_max_us") < STALL_UIKIT_MAX_US, "UIKit waited on a stalled render owner", s)
    snapshot("11-final", s, screenshot=False)
    log(f"iPhone 18 Pro simulator G3b function route passed; host load {load()}")


if __name__ == "__main__":
    main()
