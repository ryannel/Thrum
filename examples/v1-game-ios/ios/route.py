"""G3b iOS simulator route: walks the shell's function rows on one launched app.

Usage: route.py <device-udid> <state-file>. Called by check-simulator.sh after
it launches the app with THRUM_V1_ROUTE=1. Function evidence only: simulator
timing is never gate evidence, so thresholds here only separate a UIKit
callback that waited on the render owner from one that did not.
"""

from __future__ import annotations

import os
import subprocess
import sys
import time
from pathlib import Path

DEVICE, STATE = sys.argv[1], Path(sys.argv[2])
OUT = Path(os.environ.get("THRUM_V1_ROUTE_OUT", "target/v1-game-ios-sim/route"))
TOUCH = os.environ.get("THRUM_V1_TOUCH", "xcuitest")
XCTESTRUN = Path(os.environ.get("THRUM_V1_XCTESTRUN", "target/v1-game-ios-sim/uitest/ThrumV1UITests.xctestrun"))
TOUCH_WINDOW = 60.0
# A UIKit callback that waited on the 1.5 s render-owner stall would exceed this.
UIKIT_MAX_US = 50_000
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


def simctl(*args: str, background: bool = False):
    command = ["xcrun", "simctl", *args]
    if background:
        return subprocess.Popen(command, stdout=subprocess.DEVNULL)
    subprocess.run(command, check=True, timeout=90, stdout=subprocess.DEVNULL)
    return None


def post(command: str, background: bool = False):
    """Post a qualification command to the app as a Darwin notification."""
    return simctl("spawn", DEVICE, "notifyutil", "-p", f"org.thrum.v1.{command}", background=background)


def foreground() -> None:
    """Bring the running app back to the foreground (launch does not relaunch it)."""
    simctl("launch", DEVICE, "org.thrum.v1")


def snapshot(name: str, state: dict[str, str], screenshot: bool = True) -> None:
    (OUT / f"{name}.txt").write_text("".join(f"{k}={v}\n" for k, v in state.items()))
    if screenshot:
        simctl("io", DEVICE, "screenshot", str(OUT / f"{name}.png"))


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
    check(num(s, "main_max_us") < UIKIT_MAX_US, "a UIKit frame or input callback ran long", s)
    snapshot("01-startup", s)
    log(f"startup: presented={s['presented']} completed={s['completed_gpu']} "
        f"callbacks={s['display_link_callbacks']} pending<={s['max_pending_frames']} "
        f"threads display/prepare/frame/world={s['display_thread']}/{s['prepare_thread']}/"
        f"{s['frame_thread']}/{s['world_thread']}")
    return s


def stall(base: dict[str, str]) -> dict[str, str]:
    post("stall-render")
    s = wait(lambda s: num(s, "stalls") > num(base, "stalls"), 15, "the render-owner stall to end")
    check(s.get("stall_status") == "Presented", "a current-epoch stall did not present", s)
    check(num(s, "stall_targets") - num(base, "stall_targets") >= 30, "display callbacks stopped during the stall", s)
    check(num(s, "stall_main_max_us") < UIKIT_MAX_US, "UIKit waited on the stalled render owner", s)
    ticks = num(s, "stall_tick_end") - num(s, "stall_tick_start")
    check(ticks >= 30, "world ticks did not advance during the stall", s)
    s = wait(presents_after(s), 15, "presentation after the stall")
    snapshot("02-stall", s, screenshot=False)
    log(f"stall: 1.5 s inside the lease section; callbacks during stall={num(s, 'stall_targets') - num(base, 'stall_targets')}, "
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
    stall_post = post("stall-render", background=True)
    stall_post.wait(timeout=30)
    time.sleep(0.2)
    post("replace-surface")
    s = wait(lambda s: num(s, "replacements") > num(base, "replacements")
             and num(s, "release_completed") > num(base, "release_completed")
             and num(s, "presented_epoch") > num(base, "epoch") and presents_after(base)(s) and settled(s),
             20, "deferred release and recovery after a stalled replacement")
    check(num(s, "release_deferred") > num(base, "release_deferred"), "the held section did not defer release", s)
    check(num(s, "stale_dropped") > num(base, "stale_dropped"), "the revoked drawable was not dropped", s)
    check(s.get("stall_status") == "StaleDropped", "the stalled frame was not rejected as stale", s)
    check(num(s, "raced_presents") == num(base, "raced_presents"), "a present raced revocation", s)
    snapshot("05-stalled-replace", s, screenshot=False)
    log(f"stalled replacement: deferred={s['release_deferred']} stale_dropped={s['stale_dropped']} "
        f"completed releases={s['release_completed']}; presents resumed on epoch {s['presented_epoch']}")
    return s


def background(base: dict[str, str], name: str, during_stall: bool = False) -> dict[str, str]:
    if during_stall:
        post("stall-render")
        time.sleep(0.2)
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
    snapshot(f"{name}-resumed", s)
    log(f"{name}: pid {s['pid']}, world epoch {s['pause_epoch']} at tick {s['pause_tick']} paused and resumed "
        f"at tick {s['resume_tick']}; surface epoch {base['epoch']} -> {s['epoch']}; "
        f"deferred releases {s['release_deferred']}")
    return s


def gesture(test: str) -> subprocess.Popen:
    """Start one real HID gesture from the XCUITest runner on the running app."""
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


def touches(base: dict[str, str]) -> dict[str, str]:
    t0 = base
    snapshot("08-before-swipe", t0)
    s, driver = touch_window("swipe", "testSwipe",
                     lambda s: num(s, "touch_releases") > num(t0, "touch_releases")
                     and num(s, "axes_moving") > num(t0, "axes_moving") and at_rest(s),
                     "a swipe that moves and releases")
    finish(driver)
    check(s.get("touch_thread") == "main", "touches were not forwarded on UIKit main", s)
    snapshot("08-touch-swipe", s)
    log(f"touch: starts={s['touch_starts']} moving sends={s['axes_moving']} released to rest")

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
    # Hiding the detached window cancels its touch; that and any later event
    # for the old touch arrive under the old window id and count stale.
    s = wait(lambda s: num(s, "touch_stale") > num(t2, "touch_stale")
             and num(s, "old_window_touches") > num(t2, "old_window_touches"), TOUCH_WINDOW,
             "the old touch's late events counted stale")
    finish(driver)
    time.sleep(1)
    s = read()
    check(at_rest(s), "a stale release changed held input", s)
    check(num(s, "axes_moving") == num(r, "axes_moving"), "a stale event sent moving axes", s)
    snapshot("09-stale-release", s)
    log(f"replacement while held: clears={s['touch_clears']} stale={s['touch_stale']} "
        f"old-window events={s['old_window_touches']}, axes stayed at rest")

    t3 = s
    s, driver = touch_window("swipe-after", "testSwipe",
                     lambda s: num(s, "touch_releases") > num(t3, "touch_releases")
                     and num(s, "axes_moving") > num(t3, "axes_moving") and at_rest(s),
                     "a swipe after recovery")
    finish(driver)
    snapshot("10-touch-after-recovery", s)
    log(f"touch after recovery: moving sends={s['axes_moving']}")
    return s


def main() -> None:
    log(f"G3b iOS route on {DEVICE} (function evidence only)")
    s = startup()
    s = stall(s)
    s = hide(s)
    s = replace(s)
    s = stalled_replace(s)
    s = background(s, "06")
    s = background(s, "07-stall", during_stall=True)
    if TOUCH == "skip":
        log("PARTIAL: touch rows skipped (THRUM_V1_TOUCH=skip)")
        sys.exit(2)
    s = touches(s)
    check(num(s, "max_pending_frames") <= 2, "more than two submitted frames", s)
    check(num(s, "main_max_us") < UIKIT_MAX_US, "a UIKit frame or input callback ran long", s)
    snapshot("11-final", s, screenshot=False)
    log("iPhone 18 Pro simulator G3b function route passed")


if __name__ == "__main__":
    main()
