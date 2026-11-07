#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/../../.."

# Function check only. Simulator CPU and GPU timings are never gate evidence.
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

RUSTC_WRAPPER= examples/v1-game-ios/ios/build-simulator.sh
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
thrum_report="$thrum_container/tmp/thrum-v1-ios-report.txt"
run_simctl launch "$thrum_device" org.thrum.v1
for ((thrum_attempt = 0; thrum_attempt < 90; thrum_attempt++)); do
    if [[ -f "$thrum_report" ]]; then
        break
    fi
    sleep 1
done
python3 - "$thrum_report" <<'PY'
from pathlib import Path
import sys

report = Path(sys.argv[1])
if not report.exists():
    sys.exit("simulator app did not write a report within 90 seconds")
raw = report.read_text()
if raw.startswith("fault:"):
    sys.exit(raw.strip())
values = dict(item.split("=", 1) for item in raw.split(";", 1)[1].strip().split()
              if "=" in item)
for key in ("display_link_callbacks", "presented", "completed_gpu", "surface_samples"):
    if int(values.get(key, "0")) < 600:
        sys.exit(f"simulator {key} below 600: {raw.strip()}")
if "Apple iOS simulator GPU" not in raw:
    sys.exit(f"simulator did not select its Metal adapter: {raw.strip()}")
print(raw.strip())
print("iPhone 18 Pro simulator function passed: build, link, boot, Metal surface, "
      f"display-link callbacks={values['display_link_callbacks']}, "
      f"presented={values['presented']}, completed={values['completed_gpu']}")
PY
