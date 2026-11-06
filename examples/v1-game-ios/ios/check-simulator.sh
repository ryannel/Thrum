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

thrum_device=$(run_simctl list devices available --json | python3 -c '
import json, sys
devices = [device for runtime, entries in json.load(sys.stdin)["devices"].items()
           if runtime.endswith("iOS-27-2")
           for device in entries if device["name"] == "iPhone 18 Pro"]
if len(devices) != 1:
    sys.exit("expected exactly one available iOS 27.2 iPhone 18 Pro simulator")
print(devices[0]["udid"])
')

if ! run_simctl list devices booted --json | python3 -c '
import json, sys
udid = sys.argv[1]
sys.exit(0 if any(device["udid"] == udid
                  for entries in json.load(sys.stdin)["devices"].values()
                  for device in entries) else 1)
' "$thrum_device"; then
    run_simctl boot "$thrum_device"
fi
run_simctl bootstatus "$thrum_device" -b

RUSTC_WRAPPER= examples/v1-game-ios/ios/build-simulator.sh
thrum_target="${CARGO_TARGET_DIR:-$PWD/target}"
if run_simctl get_app_container "$thrum_device" org.thrum.v1 data >/dev/null 2>&1; then
    run_simctl terminate "$thrum_device" org.thrum.v1 >/dev/null 2>&1 || true
    run_simctl uninstall "$thrum_device" org.thrum.v1
else
    thrum_lookup_status=$?
    if [[ "$thrum_lookup_status" -eq 124 ]]; then
        echo "simulator app container lookup timed out" >&2
        exit 124
    fi
fi
run_simctl install "$thrum_device" "$thrum_target/v1-game-ios-sim/ThrumV1.app"
thrum_container=$(run_simctl get_app_container "$thrum_device" org.thrum.v1 data)
thrum_report="$thrum_container/tmp/thrum-v1-ios-report.txt"
rm -f -- "$thrum_report"
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
print("iPhone 18 Pro simulator function passed: build, link, boot, Metal surface, "
      f"display-link callbacks={values['display_link_callbacks']}, "
      f"presented={values['presented']}, completed={values['completed_gpu']}")
PY
