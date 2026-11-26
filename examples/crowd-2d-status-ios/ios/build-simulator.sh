#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/../../.."
exec crates/thrum-app/ios/build-simulator.sh crowd-2d-status-ios examples/crowd-2d-status-ios/ios \
    "${CARGO_TARGET_DIR:-$PWD/target}/crowd-2d-status-ios-sim"
