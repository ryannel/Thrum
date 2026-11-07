#!/usr/bin/env bash
# Build a self-hosted XCUITest runner that drives real simulator touches for
# the G3b route, without an Xcode project. Output: <target>/v1-game-ios-sim/uitest/.
set -euo pipefail
cd -- "$(dirname -- "$0")/../../.."

thrum_target="${CARGO_TARGET_DIR:-$PWD/target}"
thrum_out="$thrum_target/v1-game-ios-sim/uitest"
thrum_platform="$(xcrun --sdk iphonesimulator --show-sdk-platform-path)"
thrum_sdk="$(xcrun --sdk iphonesimulator --show-sdk-path)"
thrum_frameworks="$thrum_platform/Developer/Library/Frameworks"
thrum_min_ios="$(plutil -extract MinimumOSVersion raw -expect string examples/v1-game-ios/ios/Info.plist)"
thrum_runner="$thrum_out/ThrumV1UITests-Runner.app"
thrum_bundle="$thrum_runner/PlugIns/ThrumV1UITests.xctest"

rm -rf "$thrum_runner"
mkdir -p "$thrum_out/module-cache"
cp -R "$thrum_platform/Developer/Library/Xcode/Agents/XCTRunner.app" "$thrum_runner"
mv "$thrum_runner/XCTRunner" "$thrum_runner/ThrumV1UITests-Runner"
plutil -replace CFBundleExecutable -string ThrumV1UITests-Runner "$thrum_runner/Info.plist"
plutil -replace CFBundleName -string ThrumV1UITests-Runner "$thrum_runner/Info.plist"
plutil -replace CFBundleIdentifier -string org.thrum.v1.uitests.xctrunner "$thrum_runner/Info.plist"
mkdir -p "$thrum_bundle" "$thrum_runner/Frameworks"
for framework in XCTest XCUIAutomation XCTestCore XCTestSupport XCTAutomationSupport XCUnit Testing; do
    if [[ -d "$thrum_frameworks/$framework.framework" ]]; then
        cp -R "$thrum_frameworks/$framework.framework" "$thrum_runner/Frameworks/"
    elif [[ -d "$thrum_platform/Developer/Library/PrivateFrameworks/$framework.framework" ]]; then
        cp -R "$thrum_platform/Developer/Library/PrivateFrameworks/$framework.framework" "$thrum_runner/Frameworks/"
    fi
done
cp "$thrum_platform/Developer/usr/lib/libXCTestSwiftSupport.dylib" "$thrum_runner/Frameworks/"

SDKROOT="$thrum_sdk" CLANG_MODULE_CACHE_PATH="$thrum_out/module-cache" \
xcrun swiftc -target "arm64-apple-ios${thrum_min_ios}-simulator" -sdk "$thrum_sdk" \
    -module-cache-path "$thrum_out/module-cache" -module-name ThrumV1UITests \
    -F "$thrum_frameworks" -I "$thrum_platform/Developer/usr/lib" -L "$thrum_platform/Developer/usr/lib" \
    -Xlinker -bundle -Xlinker -rpath -Xlinker @executable_path/Frameworks \
    -Xlinker -rpath -Xlinker @loader_path/../../Frameworks \
    -framework XCTest -framework XCUIAutomation -lXCTestSwiftSupport \
    examples/v1-game-ios/ios/uitest/ThrumV1Touch.swift -o "$thrum_bundle/ThrumV1UITests"
cat > "$thrum_bundle/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
    <key>CFBundleExecutable</key><string>ThrumV1UITests</string>
    <key>CFBundleIdentifier</key><string>org.thrum.v1.uitests</string>
    <key>CFBundleName</key><string>ThrumV1UITests</string>
    <key>CFBundlePackageType</key><string>BNDL</string>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>CFBundleVersion</key><string>1</string>
    <key>CFBundleSupportedPlatforms</key><array><string>iPhoneSimulator</string></array>
</dict></plist>
PLIST
codesign --force --sign - "$thrum_bundle"
for item in "$thrum_runner/Frameworks/"*; do codesign --force --sign - "$item"; done
codesign --force --sign - "$thrum_runner"
cat > "$thrum_out/ThrumV1UITests.xctestrun" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
    <key>ThrumV1UITests</key>
    <dict>
        <key>TestHostPath</key><string>__TESTROOT__/ThrumV1UITests-Runner.app</string>
        <key>TestBundlePath</key><string>__TESTHOST__/PlugIns/ThrumV1UITests.xctest</string>
        <key>TestHostBundleIdentifier</key><string>org.thrum.v1.uitests.xctrunner</string>
        <key>IsUITestBundle</key><true/>
        <key>IsXCTRunnerHostedTestBundle</key><true/>
        <key>UITargetAppPath</key><string>$thrum_target/v1-game-ios-sim/ThrumV1.app</string>
        <key>UITargetAppBundleIdentifier</key><string>org.thrum.v1</string>
        <key>TestingEnvironmentVariables</key>
        <dict>
            <key>DYLD_FRAMEWORK_PATH</key><string>__PLATFORMS__/iPhoneSimulator.platform/Developer/Library/Frameworks</string>
            <key>DYLD_LIBRARY_PATH</key><string>__PLATFORMS__/iPhoneSimulator.platform/Developer/usr/lib</string>
        </dict>
    </dict>
    <key>__xctestrun_metadata__</key>
    <dict><key>FormatVersion</key><integer>1</integer></dict>
</dict></plist>
PLIST
printf 'Built %s\n' "$thrum_out/ThrumV1UITests.xctestrun"
