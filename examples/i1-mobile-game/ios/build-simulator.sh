#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/../../.."

thrum_rustc="$(rustup which --toolchain 1.99.1 rustc)"
export PATH="$(dirname -- "$thrum_rustc"):$PATH"
rustup run 1.99.1 cargo build --locked --offline --release --package i1-mobile-game --target aarch64-apple-ios-sim

thrum_out="$PWD/target/i1-mobile-ios-sim"
thrum_app="$thrum_out/ThrumI1.app"
mkdir -p "$thrum_app" "$thrum_out/module-cache"
cp examples/i1-mobile-game/ios/Info.plist "$thrum_app/Info.plist"
thrum_sdk="$(xcrun --sdk iphonesimulator --show-sdk-path)"
CLANG_MODULE_CACHE_PATH="$thrum_out/module-cache" \
SWIFT_MODULE_CACHE_PATH="$thrum_out/module-cache" \
xcrun swiftc -target arm64-apple-ios18.0-simulator -sdk "$thrum_sdk" \
    -module-cache-path "$thrum_out/module-cache" \
    examples/i1-mobile-game/ios/main.swift \
    target/aarch64-apple-ios-sim/release/libi1_mobile_game.a \
    -framework UIKit -framework Foundation -framework CoreFoundation \
    -framework CoreGraphics -framework QuartzCore -framework Metal \
    -lobjc -o "$thrum_app/ThrumI1"
printf 'Built %s\n' "$thrum_app"
