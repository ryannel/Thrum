// A separate LaunchServices identity for the native engine proof. No grant request.
import Foundation
import CoreGraphics

let output = URL(fileURLWithPath: CommandLine.arguments[1])
let reportURL = output.appendingPathComponent("launcher.json")
var report: [String: Any] = ["screen_recording_before": CGPreflightScreenCaptureAccess()]
func save() throws {
    try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
        .write(to: reportURL, options: .atomic)
}
do {
    try save()
    guard report["screen_recording_before"] as? Bool == false else {
        throw NSError(domain: "ThrumMacM2", code: 1, userInfo: [NSLocalizedDescriptionKey: "launcher already has Screen Recording access"])
    }
    let logURL = output.appendingPathComponent("capture.log")
    _ = FileManager.default.createFile(atPath: logURL.path, contents: nil)
    let log = try FileHandle(forWritingTo: logURL)
    let child = Process()
    child.executableURL = URL(fileURLWithPath: CommandLine.arguments[2])
    child.currentDirectoryURL = URL(fileURLWithPath: CommandLine.arguments[3])
    child.arguments = Array(CommandLine.arguments.dropFirst(4))
    var environment = ProcessInfo.processInfo.environment
    environment["PATH"] = "/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"
    child.environment = environment
    child.standardOutput = log
    child.standardError = log
    try child.run()
    report["capture_driver_pid"] = child.processIdentifier
    try save()
    child.waitUntilExit()
    report["capture_exit_code"] = child.terminationStatus
    report["screen_recording_after"] = CGPreflightScreenCaptureAccess()
    try save()
    try log.close()
    exit(child.terminationStatus)
} catch {
    report["error"] = String(describing: error)
    try? save()
    exit(1)
}
