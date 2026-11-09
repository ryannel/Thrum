// LaunchServices attributes this simulator qualification tree to its own app.
import Foundation
import CoreGraphics

let output = URL(fileURLWithPath: CommandLine.arguments[1])
let reportURL = output.appendingPathComponent("launcher.json")
var report: [String: Any] = [
    "launcher_pid": ProcessInfo.processInfo.processIdentifier,
    "launcher_executable": Bundle.main.executablePath ?? "",
    "bundle_identifier": Bundle.main.bundleIdentifier ?? "",
    "preflight_api": "CGPreflightScreenCaptureAccess",
    "screen_recording_before": CGPreflightScreenCaptureAccess(),
]
func save() throws {
    try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
        .write(to: reportURL, options: .atomic)
}
do {
    try save()
    guard report["screen_recording_before"] as? Bool == false else {
        throw NSError(domain: "ThrumCookedDraw", code: 1,
                      userInfo: [NSLocalizedDescriptionKey: "launcher has Screen Recording access"])
    }
    let logURL = output.appendingPathComponent("harness.log")
    _ = FileManager.default.createFile(atPath: logURL.path, contents: nil)
    let log = try FileHandle(forWritingTo: logURL)
    let child = Process()
    child.executableURL = URL(fileURLWithPath: CommandLine.arguments[2])
    child.currentDirectoryURL = URL(fileURLWithPath: CommandLine.arguments[3])
    child.arguments = Array(CommandLine.arguments.dropFirst(5))
    var environment = ProcessInfo.processInfo.environment
    environment["PATH"] = CommandLine.arguments[4]
    environment["THRUM_COOKED_DRAW_LAUNCHER_REPORT"] = reportURL.path
    child.environment = environment
    child.standardOutput = log
    child.standardError = log
    report["driver_executable"] = child.executableURL?.path
    report["driver_arguments"] = child.arguments
    try child.run()
    report["driver_pid"] = child.processIdentifier
    try save()
    child.waitUntilExit()
    report["driver_exit_code"] = child.terminationStatus
    report["screen_recording_after"] = CGPreflightScreenCaptureAccess()
    try save()
    try log.close()
    guard report["screen_recording_after"] as? Bool == false else { exit(1) }
    exit(child.terminationStatus)
} catch {
    report["error"] = String(describing: error)
    try? save()
    exit(1)
}
