#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/../../.."

thrum_rustc="$(rustup which --toolchain 1.99.1 rustc)"
export PATH="$(dirname -- "$thrum_rustc"):$PATH"
export RUSTC="$thrum_rustc"
thrum_plist=examples/v1-game-ios/ios/Info.plist
thrum_min_ios="$(plutil -extract MinimumOSVersion raw -expect string "$thrum_plist")"
export IPHONEOS_DEPLOYMENT_TARGET="$thrum_min_ios"
rustup run 1.99.1 cargo build --locked --offline --release --package v1-game-ios --lib --target aarch64-apple-ios-sim

thrum_target="${CARGO_TARGET_DIR:-$PWD/target}"
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
    examples/v1-game-ios/ios/main.swift \
    "$thrum_target/aarch64-apple-ios-sim/release/libv1_game_ios.a" \
    -framework UIKit -framework Foundation -framework CoreFoundation \
    -framework CoreGraphics -framework QuartzCore -framework Metal \
    -lobjc -o "$thrum_app/ThrumV1"
printf 'Built %s\n' "$thrum_app"
