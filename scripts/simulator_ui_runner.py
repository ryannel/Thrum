"""Build a self-hosted XCUITest runner for an existing simulator app."""

import os
from pathlib import Path
import plistlib
import shutil
import subprocess


def build(app, source, output, suite):
    output.mkdir(parents=True, exist_ok=True)
    platform = Path(subprocess.check_output(
        ["xcrun", "--sdk", "iphonesimulator", "--show-sdk-platform-path"], text=True).strip())
    sdk = subprocess.check_output(
        ["xcrun", "--sdk", "iphonesimulator", "--show-sdk-path"], text=True).strip()
    developer = platform / "Developer"
    frameworks = developer / "Library/Frameworks"
    private = developer / "Library/PrivateFrameworks"
    developer_lib = developer / "usr/lib"

    application = plistlib.loads((app / "Info.plist").read_bytes())
    bundle_id = application["CFBundleIdentifier"]
    runner_id = f"{bundle_id}.uitests.xctrunner"
    runner = output / f"{suite}-Runner.app"
    shutil.copytree(developer / "Library/Xcode/Agents/XCTRunner.app", runner, symlinks=True)
    runner_plist = runner / "Info.plist"
    info = plistlib.loads(runner_plist.read_bytes())
    executable = f"{suite}-Runner"
    (runner / "XCTRunner").rename(runner / executable)
    runner_plist.write_bytes(plistlib.dumps({
        **info, "CFBundleIdentifier": runner_id, "CFBundleExecutable": executable,
        "CFBundleName": executable,
    }))
    bundle = runner / "PlugIns" / f"{suite}.xctest"
    bundle.mkdir(parents=True)
    (runner / "Frameworks").mkdir(exist_ok=True)
    for name in ("XCTest", "XCUIAutomation", "XCTestCore", "XCTestSupport",
                 "XCTAutomationSupport", "XCUnit", "Testing"):
        for location in (frameworks, private):
            library = location / f"{name}.framework"
            if library.is_dir():
                shutil.copytree(library, runner / "Frameworks" / library.name, symlinks=True)
                break
    support = developer_lib / "libXCTestSwiftSupport.dylib"
    shutil.copy2(support, runner / "Frameworks" / support.name)

    minimum = application["MinimumOSVersion"]
    subprocess.run([
        "xcrun", "swiftc", "-target", f"arm64-apple-ios{minimum}-simulator", "-sdk", sdk,
        "-module-cache-path", str(output / "module-cache"), "-module-name", suite,
        "-F", str(frameworks), "-I", str(developer_lib), "-L", str(developer_lib),
        "-Xlinker", "-bundle", "-Xlinker", "-rpath", "-Xlinker", "@executable_path/Frameworks",
        "-Xlinker", "-rpath", "-Xlinker", "@loader_path/../../Frameworks",
        "-framework", "XCTest", "-framework", "XCUIAutomation", "-lXCTestSwiftSupport",
        str(source), "-o", str(bundle / suite),
    ], env=dict(os.environ, SDKROOT=sdk), check=True)
    (bundle / "Info.plist").write_bytes(plistlib.dumps({
        "CFBundleExecutable": suite, "CFBundleName": suite, "CFBundlePackageType": "BNDL",
        "CFBundleIdentifier": f"{bundle_id}.uitests", "CFBundleVersion": "1",
        "CFBundleSupportedPlatforms": ["iPhoneSimulator"],
    }))
    for item in [bundle, *(runner / "Frameworks").iterdir(), runner]:
        subprocess.run(["codesign", "--force", "--sign", "-", item], check=True)
    config = output / f"{suite}.xctestrun"
    config.write_bytes(plistlib.dumps({
        suite: {
            "TestHostPath": f"__TESTROOT__/{runner.name}",
            "TestBundlePath": f"__TESTHOST__/PlugIns/{bundle.name}",
            "TestHostBundleIdentifier": runner_id,
            "IsUITestBundle": True,
            "IsXCTRunnerHostedTestBundle": True,
            "UITargetAppPath": str(app),
            "UITargetAppBundleIdentifier": bundle_id,
            "TestingEnvironmentVariables": {
                "DYLD_FRAMEWORK_PATH": "__PLATFORMS__/iPhoneSimulator.platform/Developer/Library/Frameworks",
                "DYLD_LIBRARY_PATH": "__PLATFORMS__/iPhoneSimulator.platform/Developer/usr/lib",
            },
        },
        "__xctestrun_metadata__": {"FormatVersion": 1},
    }))
    return config
