// LaunchServices supplies this app's own TCC identity. Never request a grant.
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
        throw NSError(domain: "ThrumCapture", code: 1, userInfo: [NSLocalizedDescriptionKey: "launcher has Screen Recording access"])
    }
    let logURL = output.appendingPathComponent("chrome.log")
    _ = FileManager.default.createFile(atPath: logURL.path, contents: nil)
    let log = try FileHandle(forWritingTo: logURL)
    let chrome = Process()
    chrome.executableURL = URL(fileURLWithPath: CommandLine.arguments[2])
    chrome.arguments = Array(CommandLine.arguments.dropFirst(3))
    chrome.standardOutput = log
    chrome.standardError = log
    try chrome.run()
    report["chrome_pid"] = chrome.processIdentifier
    try save()
    chrome.waitUntilExit()
    report["chrome_exit_code"] = chrome.terminationStatus
    report["screen_recording_after"] = CGPreflightScreenCaptureAccess()
    try save()
    try log.close()
    exit(chrome.terminationStatus)
} catch {
    report["error"] = String(describing: error)
    try? save()
    exit(1)
}
