#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/../../.."

# Function check only. Simulator CPU and GPU timings are never gate evidence.
#
# The route builds the app, boots a fresh iPhone 18 Pro simulator owned by this
# run, launches the game with qualification commands enabled and walks the G3b
# rows: startup, a render-owner stall, a hidden view, surface replacement (plain
# and during a stall), a real background/foreground round trip (plain and during
# a stall) and touch. Touch is a real HID gesture from a self-hosted XCUITest
# runner (simctl has none) attached to the running process.
# THRUM_V1_TOUCH=skip runs the other rows and exits 2 (partial).
# THRUM_V1_ROUTE_OUT names a directory for raw state snapshots and screenshots.
run_simctl() {
    python3 - "$@" <<'PY'
import subprocess
import sys

command = ["xcrun", "simctl", *sys.argv[1:]]
try:
    result = subprocess.run(command, check=False, timeout=90)
except subprocess.TimeoutExpired:
    print(f"simctl {sys.argv[1]} did not finish within 90 seconds", file=sys.stderr)
    sys.exit(124)
sys.exit(result.returncode)
PY
}

thrum_rustc="$(rustup which --toolchain 1.99.1 rustc)"
PATH="$(dirname -- "$thrum_rustc"):$PATH" RUSTC="$thrum_rustc" RUSTC_WRAPPER= \
IPHONEOS_DEPLOYMENT_TARGET="$(plutil -extract MinimumOSVersion raw -expect string examples/v1-game-ios/ios/Info.plist)" \
    rustup run 1.99.1 cargo clippy --locked --offline --package v1-game-ios \
    --target aarch64-apple-ios-sim --all-targets -- -D warnings
RUSTC_WRAPPER= examples/v1-game-ios/ios/build-simulator.sh
examples/v1-game-ios/ios/build-uitest.sh >/dev/null
thrum_target="${CARGO_TARGET_DIR:-$PWD/target}"
thrum_app="$thrum_target/v1-game-ios-sim/ThrumV1.app"
if [[ "$(plutil -extract CADisableMinimumFrameDurationOnPhone raw -expect bool "$thrum_app/Info.plist")" != true ]]; then
    echo "built iPhone app does not enable its display link's full refresh-rate range" >&2
    exit 1
fi

# A shared booted simulator can have a wedged app-container service or a live
# installation. Qualify this build on a fresh device owned by this run.
thrum_device=$(run_simctl create "Thrum V1 function $$" \
    com.apple.CoreSimulator.SimDeviceType.iPhone-18-Pro \
    com.apple.CoreSimulator.SimRuntime.iOS-27-2)
if [[ ! "$thrum_device" =~ ^[[:xdigit:]]{8}(-[[:xdigit:]]{4}){3}-[[:xdigit:]]{12}$ ]]; then
    echo "simctl create did not return a device UUID: $thrum_device" >&2
    exit 1
fi
cleanup() {
    local status=$?
    run_simctl shutdown "$thrum_device" >/dev/null 2>&1 || true
    if ! run_simctl delete "$thrum_device"; then
        echo "failed to delete owned simulator $thrum_device" >&2
        if [[ "$status" -eq 0 ]]; then status=1; fi
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
run_simctl bootstatus "$thrum_device" -b
run_simctl install "$thrum_device" "$thrum_app"
thrum_container=$(run_simctl get_app_container "$thrum_device" org.thrum.v1 data)
# A just-booted device can stall its first launch while it finishes setting up.
# A second launch of a running app only foregrounds the same process, so one
# retry cannot start a second game.
if ! SIMCTL_CHILD_THRUM_V1_ROUTE=1 run_simctl launch "$thrum_device" org.thrum.v1; then
    echo "first launch did not complete; retrying once" >&2
    SIMCTL_CHILD_THRUM_V1_ROUTE=1 run_simctl launch "$thrum_device" org.thrum.v1
fi
python3 examples/v1-game-ios/ios/route.py "$thrum_device" "$thrum_container/tmp/thrum-v1-ios-state.txt"
