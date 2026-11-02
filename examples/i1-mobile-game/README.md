# I1 mobile game shell

One bundled tap counter (`i1-tap-counter-v1`, package version `0.1.0`) uses the existing `HeadlessHost`, authority, replica and local receipt path. A touch release commits an action; the display color follows the replica's committed counter. Red means the local world clock is overloaded, orange means input admission is full, and black means a terminal shell fault. The color response is a lifecycle and delivery proof, not the later game renderer.

## iOS simulator package

On macOS with Xcode, the pinned Rust iOS simulator target and offline Cargo cache:

```sh
bash examples/i1-mobile-game/ios/build-simulator.sh
```

The unsigned app is written under `target/i1-mobile-ios-sim/ThrumI1.app`. Signing, installation and running on a simulator or iPhone are separate checks.

## Android debug package

With Android SDK, NDK 28.2.13676358, Gradle 8.13, JDK 17, the pinned Rust Android target and offline Cargo cache:

```sh
bash examples/i1-mobile-game/android/build-debug.sh
```

Set `ANDROID_HOME`; `ANDROID_NDK_HOME` may override the selected SDK NDK path. The script selects the NDK LLVM toolchain and API 28 Rust target linker, builds `libi1_mobile_game.so`, places it under the ignored Gradle build directory, and assembles a debug APK. The Java Activity uses the `android-activity` 0.6.1 matching GameActivity 4.4.0 dependency and forwards foreground transitions through JNI. This package has not been assembled or run locally yet.

The I1 shell is local play only. Later multiplayer clients must leave the remote authority running independently of the phone surface.
