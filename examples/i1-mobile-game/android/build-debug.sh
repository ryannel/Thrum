#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/../../.."

thrum_rustc="$(rustup which --toolchain 1.99.1 rustc)"
export PATH="$(dirname -- "$thrum_rustc"):$PATH"
: "${ANDROID_HOME:?Set ANDROID_HOME to the Android SDK root}"
thrum_ndk="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/28.2.13676358}"
shopt -s nullglob
thrum_bins=("$thrum_ndk"/toolchains/llvm/prebuilt/*/bin)
if (( ${#thrum_bins[@]} != 1 )); then
    printf 'Expected one NDK LLVM host toolchain under %s\n' "$thrum_ndk" >&2
    exit 1
fi
export PATH="${thrum_bins[0]}:$PATH"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="${thrum_bins[0]}/aarch64-linux-android28-clang"
rustup run 1.99.1 cargo build --locked --offline --release --package i1-mobile-game --target aarch64-linux-android

thrum_android="$PWD/examples/i1-mobile-game/android"
thrum_jni="$thrum_android/app/build/generated/jniLibs/arm64-v8a"
mkdir -p "$thrum_jni"
cp target/aarch64-linux-android/release/libi1_mobile_game.so "$thrum_jni/"

cd "$thrum_android"
gradle --no-daemon :app:assembleDebug
