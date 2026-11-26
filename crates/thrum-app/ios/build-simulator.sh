#!/usr/bin/env bash
# Package one game's static library with the shared UIKit bootstrap.
set -euo pipefail
cd -- "$(dirname -- "$0")/../../.."

if (( $# < 3 || $# > 5 )); then
    echo "usage: build-simulator.sh PACKAGE APP_SOURCE OUTPUT [CARGO_FEATURE [SWIFT_DEFINE]]" >&2
    exit 2
fi
package="$1"
source="$2"
output="$3"
cargo_features=()
swift_defines=()
cargo_target="${CARGO_TARGET_DIR:-$PWD/target}"
if [[ -n "${4:-}" ]]; then
    cargo_features=(--features "$4")
    cargo_target="$cargo_target/$package-$4"
fi
if [[ -n "${5:-}" ]]; then
    swift_defines=(-D "$5")
fi

rustc="$(rustup which --toolchain stable rustc)"
export PATH="$(dirname -- "$rustc"):$PATH"
export RUSTC="$rustc"
plist="$source/Info.plist"
min_ios="$(plutil -extract MinimumOSVersion raw -expect string "$plist")"
export IPHONEOS_DEPLOYMENT_TARGET="$min_ios"
name="$(plutil -extract CFBundleExecutable raw -expect string "$plist")"
app="$output/$name.app"
rustup run stable cargo build --locked --offline --release --package "$package" --lib \
    --target aarch64-apple-ios-sim --target-dir "$cargo_target" ${cargo_features[@]+"${cargo_features[@]}"}
mkdir -p "$app" "$output/module-cache"
cp "$plist" "$app/Info.plist"

sdk="$(xcrun --sdk iphonesimulator --show-sdk-path)"
SDKROOT="$sdk" CLANG_MODULE_CACHE_PATH="$output/module-cache" SWIFT_MODULE_CACHE_PATH="$output/module-cache" \
xcrun swiftc -target "arm64-apple-ios${min_ios}-simulator" -sdk "$sdk" \
    -module-cache-path "$output/module-cache" \
    ${swift_defines[@]+"${swift_defines[@]}"} crates/thrum-app/ios/main.swift "$source/application.swift" \
    "$cargo_target/aarch64-apple-ios-sim/release/lib${package//-/_}.a" \
    -framework UIKit -framework Foundation -framework CoreFoundation \
    -framework CoreGraphics -framework QuartzCore -framework Metal \
    -lobjc -o "$app/$name"
printf 'Built %s\n' "$app"
