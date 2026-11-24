#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/../../.."
case "${1:-}" in
    --prepare|--device-worker) ;;
    *) exec python3 scripts/v1_simulator_route.py "$@" ;;
esac

# Function check only. Simulator CPU and GPU timings are never gate evidence.
#
# The simulator supervisor clears and boots its owned iPhone 18 Pro. The
# worker launches the game with qualification commands enabled and walks the G3b
# rows: startup, a render-owner stall, a hidden view, surface replacement (plain
# and during a stall), a real background/foreground round trip (plain and during
# a stall), touch, and rotation (portrait → landscape → portrait, captured to a
# 20 fps frame strip). Touch and rotation are driven by a self-hosted XCUITest
# runner (simctl has neither) attached to the running process.
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

if [[ "$1" == "--prepare" ]]; then
    thrum_rustc="$(rustup which --toolchain stable rustc)"
    PATH="$(dirname -- "$thrum_rustc"):$PATH" RUSTC="$thrum_rustc" RUSTC_WRAPPER= \
    IPHONEOS_DEPLOYMENT_TARGET="$(plutil -extract MinimumOSVersion raw -expect string examples/v1-game-ios/ios/Info.plist)" \
        rustup run stable cargo clippy --locked --offline --package v1-game-ios --features proving \
        --target aarch64-apple-ios-sim --all-targets -- -D warnings
    RUSTC_WRAPPER= examples/v1-game-ios/ios/build-simulator.sh --proving
    examples/v1-game-ios/ios/build-uitest.sh >/dev/null
    exit 0
fi

thrum_target="${CARGO_TARGET_DIR:-$PWD/target}"
thrum_app="$thrum_target/v1-game-ios-sim/ThrumV1.app"
export THRUM_V1_XCTESTRUN="$thrum_target/v1-game-ios-sim/uitest/ThrumV1UITests.xctestrun"
if [[ "$(plutil -extract CADisableMinimumFrameDurationOnPhone raw -expect bool "$thrum_app/Info.plist")" != true ]]; then
    echo "built iPhone app does not enable its display link's full refresh-rate range" >&2
    exit 1
fi

thrum_device=$(python3 scripts/simulator_lifecycle.py worker-device)
mkdir -- "${THRUM_V1_ROUTE_OUT:?Use a fresh route directory}"
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
