#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/../../.."

output="${CARGO_TARGET_DIR:-$PWD/target}/v1-game-ios-sim"
case "${1:-}" in
    "") exec crates/thrum-app/ios/build-simulator.sh v1-game-ios examples/v1-game-ios/ios "$output" ;;
    --proving)
        crates/thrum-app/ios/build-simulator.sh v1-game-ios examples/v1-game-ios/ios "$output" proving THRUM_PROVING
        plutil -insert CFBundleURLTypes -json '[{"CFBundleURLName":"org.thrum.v1.invite","CFBundleURLSchemes":["thrum-v1"]}]' \
            "$output/ThrumV1.app/Info.plist"
        ;;
    *) echo "usage: build-simulator.sh [--proving]" >&2; exit 2 ;;
esac
