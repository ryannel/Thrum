#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/../../.."

cargo_features=()
swift_defines=()
case "${1:-}" in
    "") ;;
    --proving)
        cargo_features=(--features proving)
        swift_defines=(-D THRUM_PROVING)
        ;;
    *) echo "usage: build-simulator.sh [--proving]" >&2; exit 2 ;;
esac

thrum_rustc="$(rustup which --toolchain stable rustc)"
export PATH="$(dirname -- "$thrum_rustc"):$PATH"
export RUSTC="$thrum_rustc"
thrum_plist=examples/v1-game-ios/ios/Info.plist
thrum_min_ios="$(plutil -extract MinimumOSVersion raw -expect string "$thrum_plist")"
export IPHONEOS_DEPLOYMENT_TARGET="$thrum_min_ios"
thrum_target="${CARGO_TARGET_DIR:-$PWD/target}"
# Keep the archive handed to Swift isolated from builds of the other variant.
thrum_cargo_target="$thrum_target"
if [[ "${1:-}" == "--proving" ]]; then
    thrum_cargo_target="$thrum_target/v1-game-ios-proving"
fi
rustup run stable cargo build --locked --offline --release --package v1-game-ios --lib --target aarch64-apple-ios-sim \
    --target-dir "$thrum_cargo_target" ${cargo_features[@]+"${cargo_features[@]}"}
thrum_out="$thrum_target/v1-game-ios-sim"
thrum_app="$thrum_out/ThrumV1.app"
mkdir -p "$thrum_app" "$thrum_out/module-cache"
cp "$thrum_plist" "$thrum_app/Info.plist"
thrum_sdk="$(xcrun --sdk iphonesimulator --show-sdk-path)"
SDKROOT="$thrum_sdk" \
CLANG_MODULE_CACHE_PATH="$thrum_out/module-cache" \
SWIFT_MODULE_CACHE_PATH="$thrum_out/module-cache" \
xcrun swiftc -target "arm64-apple-ios${thrum_min_ios}-simulator" -sdk "$thrum_sdk" \
    -module-cache-path "$thrum_out/module-cache" \
    ${swift_defines[@]+"${swift_defines[@]}"} examples/v1-game-ios/ios/main.swift \
    "$thrum_cargo_target/aarch64-apple-ios-sim/release/libv1_game_ios.a" \
    -framework UIKit -framework Foundation -framework CoreFoundation \
    -framework CoreGraphics -framework QuartzCore -framework Metal \
    -lobjc -o "$thrum_app/ThrumV1"
printf 'Built %s\n' "$thrum_app"
