#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/../../.."

thrum_rustc="$(rustup which --toolchain 1.99.1 rustc)"
export PATH="$(dirname -- "$thrum_rustc"):$PATH"
rustup run 1.99.1 cargo build --locked --offline --release --package i1-mobile-game --target aarch64-linux-android

thrum_android="$PWD/examples/i1-mobile-game/android"
thrum_jni="$thrum_android/app/build/generated/jniLibs/arm64-v8a"
mkdir -p "$thrum_jni"
cp target/aarch64-linux-android/release/libi1_mobile_game.so "$thrum_jni/"

cd "$thrum_android"
gradle --no-daemon :app:assembleDebug
