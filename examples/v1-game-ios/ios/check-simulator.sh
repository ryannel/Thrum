#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/../../.."

# Function check only. Simulator CPU and GPU timings are never gate evidence.
thrum_device=$(xcrun simctl list devices available --json | python3 -c '
import json, sys
devices = [device for runtime, entries in json.load(sys.stdin)["devices"].items()
           if runtime.endswith("iOS-27-2")
           for device in entries if device["name"] == "iPhone 18 Pro"]
if len(devices) != 1:
    sys.exit("expected exactly one available iOS 27.2 iPhone 18 Pro simulator")
print(devices[0]["udid"])
')

if ! xcrun simctl list devices booted --json | python3 -c '
import json, sys
udid = sys.argv[1]
sys.exit(0 if any(device["udid"] == udid
                  for entries in json.load(sys.stdin)["devices"].values()
                  for device in entries) else 1)
' "$thrum_device"; then
    xcrun simctl boot "$thrum_device"
fi
xcrun simctl bootstatus "$thrum_device" -b

RUSTC_WRAPPER= examples/v1-game-ios/ios/build-simulator.sh
thrum_target="${CARGO_TARGET_DIR:-$PWD/target}"
if xcrun simctl get_app_container "$thrum_device" org.thrum.v1 data >/dev/null 2>&1; then
    xcrun simctl terminate "$thrum_device" org.thrum.v1 >/dev/null 2>&1 || true
    xcrun simctl uninstall "$thrum_device" org.thrum.v1
fi
xcrun simctl install "$thrum_device" "$thrum_target/v1-game-ios-sim/ThrumV1.app"
xcrun simctl launch "$thrum_device" org.thrum.v1
thrum_container=$(xcrun simctl get_app_container "$thrum_device" org.thrum.v1 data)
thrum_report="$thrum_container/tmp/thrum-v1-ios-report.txt"
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
