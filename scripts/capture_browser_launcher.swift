// LaunchServices supplies this app's own TCC identity. Never request a grant.
import Foundation
import CoreGraphics
import Darwin

let output = URL(fileURLWithPath: CommandLine.arguments[1])
let reportURL = output.appendingPathComponent("launcher.json")
var report: [String: Any] = ["screen_recording_before": CGPreflightScreenCaptureAccess()]
func save() throws {
    try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
        .write(to: reportURL, options: .atomic)
}
// Retain the exited leader until all group members are gone. Its PID cannot
// be recycled while any escalation can still address this process group.
func groupHasHelpers(_ leader: pid_t) throws -> Bool {
    var query = [CTL_KERN, KERN_PROC, KERN_PROC_PGRP, leader]
    var size = 0
    for _ in 0..<4 {
        guard sysctl(&query, u_int(query.count), nil, &size, nil, 0) == 0 else {
            throw POSIXError(POSIXErrorCode(rawValue: errno) ?? .EIO)
        }
        let capacity = size / MemoryLayout<kinfo_proc>.stride + 8
        var processes = [kinfo_proc](repeating: kinfo_proc(), count: capacity)
        size = capacity * MemoryLayout<kinfo_proc>.stride
        if sysctl(&query, u_int(query.count), &processes, &size, nil, 0) == 0 {
            return processes.prefix(size / MemoryLayout<kinfo_proc>.stride)
                .contains { $0.kp_proc.p_pid != leader }
        }
        if errno != ENOMEM { break }
    }
    throw POSIXError(POSIXErrorCode(rawValue: errno) ?? .EIO)
}

func check(_ result: Int32) throws {
    if result != 0 { throw POSIXError(POSIXErrorCode(rawValue: result) ?? .EIO) }
}

func launchInGroup(_ arguments: [String], log: FileHandle) throws -> pid_t {
    var attributes: posix_spawnattr_t?
    try check(posix_spawnattr_init(&attributes))
    defer { posix_spawnattr_destroy(&attributes) }
    try check(posix_spawnattr_setflags(&attributes, Int16(POSIX_SPAWN_SETPGROUP)))
    try check(posix_spawnattr_setpgroup(&attributes, 0))
    var actions: posix_spawn_file_actions_t?
    try check(posix_spawn_file_actions_init(&actions))
    defer { posix_spawn_file_actions_destroy(&actions) }
    try check(posix_spawn_file_actions_adddup2(&actions, log.fileDescriptor, STDOUT_FILENO))
    try check(posix_spawn_file_actions_adddup2(&actions, log.fileDescriptor, STDERR_FILENO))
    var argv = arguments.map { strdup($0) }
    defer { argv.forEach { free($0) } }
    argv.append(nil)
    var pid: pid_t = 0
    try check(posix_spawn(&pid, arguments[0], &actions, &attributes, &argv, environ))
    return pid
}

func supervise(_ arguments: [String], log: FileHandle, control: URL) throws -> Int32 {
    let request = control.appendingPathComponent("request")
    func phase() -> String? { try? String(contentsOf: request, encoding: .utf8) }
    if phase() != nil {
        // A startup timeout can precede LaunchServices delivering the launch.
        report["owned_processes_gone"] = true
        throw POSIXError(.ECANCELED)
    }
    let pid = try launchInGroup(arguments, log: log)
    var reaped = false
    defer {
        if !reaped {
            _ = kill(-pid, SIGKILL)
            while waitpid(pid, nil, 0) < 0 && errno == EINTR {}
        }
    }
    report["chrome_pid"] = pid
    try save()
    var requested = false
    var escalation = [String]()
    var termination = siginfo_t()
    while true {
        let command = phase()
        if termination.si_pid == 0 {
            if waitid(P_PID, id_t(pid), &termination, WEXITED | WNOHANG | WNOWAIT) < 0 {
                if errno == EINTR { continue }
                throw POSIXError(POSIXErrorCode(rawValue: errno) ?? .EIO)
            }
            if termination.si_pid != 0 {
                report["chrome_exited"] = true
                try save()
            }
        }
        if try termination.si_pid != 0 && !groupHasHelpers(pid) { break }
        if let command {
            if !requested && termination.si_pid == 0 {
                requested = true
                report["shutdown_requested"] = true
                try save()
            }
            if command == "term" || command == "kill", !escalation.contains(command) {
                if kill(-pid, command == "term" ? SIGTERM : SIGKILL) == 0 {
                    escalation.append(command)
                    report["shutdown_escalation"] = escalation
                    try save()
                } else if errno != ESRCH {
                    throw POSIXError(POSIXErrorCode(rawValue: errno) ?? .EIO)
                }
            }
        }
        Thread.sleep(forTimeInterval: 0.05)
    }
    var waited: pid_t
    repeat { waited = waitpid(pid, nil, 0) } while waited < 0 && errno == EINTR
    guard waited == pid else { throw POSIXError(POSIXErrorCode(rawValue: errno) ?? .EIO) }
    reaped = true
    report["owned_processes_gone"] = true
    if termination.si_code == CLD_EXITED {
        report["chrome_exit_code"] = termination.si_status
    } else {
        report["chrome_exit_signal"] = termination.si_status
    }
    try save()
    return termination.si_status
}

let owned = CommandLine.arguments[2] == "--owned"
do {
    try save()
    guard report["screen_recording_before"] as? Bool == false else {
        throw NSError(domain: "ThrumCapture", code: 1, userInfo: [NSLocalizedDescriptionKey: "launcher has Screen Recording access"])
    }
    let logURL = output.appendingPathComponent("chrome.log")
    _ = FileManager.default.createFile(atPath: logURL.path, contents: nil)
    let log = try FileHandle(forWritingTo: logURL)
    if owned {
        let control = URL(fileURLWithPath: CommandLine.arguments[3])
        let status = try supervise(Array(CommandLine.arguments.dropFirst(4)), log: log, control: control)
        try log.close()
        report["screen_recording_after"] = CGPreflightScreenCaptureAccess()
        report["chrome_final"] = true
        try save()
        exit(status)
    }
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
    if owned {
        if report["chrome_pid"] == nil { report["owned_processes_gone"] = true }
        report["chrome_final"] = true
    }
    try? save()
    exit(1)
}
