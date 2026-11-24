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

// Small newline-delimited messages travel on loopback even if the capture
// volume is full. Chrome cannot inherit the supervisor's socket.
final class LauncherControl {
    private var fd: Int32 = -1
    private var input = Data()
    var connected: Bool { fd >= 0 }

    init(port: UInt16, token: String) throws {
        fd = socket(AF_INET, SOCK_STREAM, 0)
        guard connected else { throw POSIXError(.EIO) }
        do {
            guard fcntl(fd, F_SETFD, FD_CLOEXEC) == 0 else { throw POSIXError(.EIO) }
            var enabled: Int32 = 1
            guard setsockopt(fd, SOL_SOCKET, SO_NOSIGPIPE, &enabled, socklen_t(MemoryLayout<Int32>.size)) == 0 else {
                throw POSIXError(.EIO)
            }
            var address = sockaddr_in()
            address.sin_len = UInt8(MemoryLayout<sockaddr_in>.size)
            address.sin_family = sa_family_t(AF_INET)
            address.sin_port = port.bigEndian
            address.sin_addr.s_addr = inet_addr("127.0.0.1")
            let result = withUnsafePointer(to: &address) {
                $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                    connect(fd, $0, socklen_t(MemoryLayout<sockaddr_in>.size))
                }
            }
            guard result == 0 else { throw POSIXError(.ECONNREFUSED) }
            sendLine(Data(token.utf8))
            let deadline = ProcessInfo.processInfo.systemUptime + 5
            repeat {
                if command() == "start", connected { return }
                Thread.sleep(forTimeInterval: 0.05)
            } while connected && ProcessInfo.processInfo.systemUptime < deadline
            throw POSIXError(.ECANCELED)
        } catch {
            disconnect()
            throw error
        }
    }

    deinit { disconnect() }

    private func disconnect() {
        if connected { close(fd); fd = -1 }
    }

    func sendLine(_ data: Data) {
        guard connected else { return }
        var line = data
        line.append(10)
        let count = line.withUnsafeBytes { send(fd, $0.baseAddress, $0.count, MSG_DONTWAIT) }
        // A truncated message cannot be acknowledged. Disconnect and let
        // the supervisor finish locally, with the on-disk receipt as backup.
        if count != line.count { disconnect() }
    }

    func command() -> String? {
        if let newline = input.firstIndex(of: 10) {
            let line = String(decoding: input[..<newline], as: UTF8.self)
            input.removeSubrange(input.startIndex...newline)
            return line
        }
        guard connected else { return nil }
        var bytes = [UInt8](repeating: 0, count: 128)
        let count = recv(fd, &bytes, bytes.count, MSG_DONTWAIT)
        if count > 0 {
            input.append(contentsOf: bytes.prefix(count))
            if input.contains(10) { return command() }
            if input.count > 128 { disconnect() }
        } else if count == 0 || (errno != EAGAIN && errno != EWOULDBLOCK && errno != EINTR) {
            disconnect()
        }
        return nil
    }

    func publish() {
        if let data = try? JSONSerialization.data(withJSONObject: report) { sendLine(data) }
        try? save()
    }
}

func supervise(_ arguments: [String], log: FileHandle, control: LauncherControl) throws -> Int32 {
    if control.command() != nil || !control.connected { throw POSIXError(.ECANCELED) }
    let pid = try launchInGroup(arguments, log: log)
    var reaped = false
    defer {
        if !reaped {
            _ = kill(-pid, SIGKILL)
            while waitpid(pid, nil, 0) < 0 && errno == EINTR {}
        }
    }
    report["chrome_pid"] = pid
    report["shutdown_requested"] = false
    control.publish()
    var disconnectedAt: TimeInterval?
    var requested = false
    var escalation = [String]()
    var termination = siginfo_t()
    while true {
        let command = control.command()
        var escalationPhase = command
        if !control.connected {
            // The owner may have crashed or run out of disk. Give Chrome a
            // short close window, then finish using the supervisor's clock.
            let now = ProcessInfo.processInfo.systemUptime
            if disconnectedAt == nil { disconnectedAt = now }
            let elapsed = now - (disconnectedAt ?? now)
            if elapsed >= 4 { escalationPhase = "kill" }
            else if elapsed >= 1 { escalationPhase = "term" }
        }
        if termination.si_pid == 0 {
            var result: Int32
            repeat {
                result = waitid(P_PID, id_t(pid), &termination, WEXITED | WNOHANG | WNOWAIT)
            } while result < 0 && errno == EINTR
            if result < 0 {
                throw POSIXError(POSIXErrorCode(rawValue: errno) ?? .EIO)
            }
            if termination.si_pid != 0 {
                report["chrome_exited"] = true
                control.publish()
            }
        }
        if try termination.si_pid != 0 && !groupHasHelpers(pid) { break }
        // Only the harness can request shutdown. EOF escalation can reap
        // Chrome during validation without qualifying its exit as requested.
        if command == "close", !requested, termination.si_pid == 0 {
            requested = true
            report["shutdown_requested"] = true
            control.publish()
        }
        if let phase = escalationPhase, phase == "term" || phase == "kill", !escalation.contains(phase) {
            if kill(-pid, phase == "term" ? SIGTERM : SIGKILL) == 0 {
                escalation.append(phase)
                report["shutdown_escalation"] = escalation
                control.publish()
            } else if errno != ESRCH {
                throw POSIXError(POSIXErrorCode(rawValue: errno) ?? .EIO)
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
    return termination.si_status
}

func openLog() throws -> FileHandle {
    let logURL = output.appendingPathComponent("chrome.log")
    _ = FileManager.default.createFile(atPath: logURL.path, contents: nil)
    return try FileHandle(forWritingTo: logURL)
}

func requireDeniedPreflight() throws {
    guard report["screen_recording_before"] as? Bool == false else {
        throw NSError(domain: "ThrumCapture", code: 1, userInfo: [NSLocalizedDescriptionKey: "launcher has Screen Recording access"])
    }
}

func runOwned(_ args: [String]) throws -> Int32 {
    guard let port = UInt16(args[3]) else { throw POSIXError(.EINVAL) }
    let control = try LauncherControl(port: port, token: args[4])
    let profile = URL(fileURLWithPath: args[5])
    var profileCreated = false
    var status: Int32 = 1
    do {
        try requireDeniedPreflight()
        let log = try openLog()
        // The supervisor creates the profile only after admission and
        // removes it even if the harness loses the control socket.
        guard mkdir(profile.path, 0o700) == 0 else {
            throw POSIXError(POSIXErrorCode(rawValue: errno) ?? .EIO)
        }
        profileCreated = true
        status = try supervise(Array(args.dropFirst(6)), log: log, control: control)
        try log.close()
    } catch {
        report["error"] = String(describing: error)
        if report["chrome_pid"] == nil { report["owned_processes_gone"] = true }
    }
    if !profileCreated {
        report["profile_removed"] = true
    } else if report["owned_processes_gone"] as? Bool == true {
        do {
            try FileManager.default.removeItem(at: profile)
            report["profile_removed"] = true
        } catch {
            report["profile_error"] = String(describing: error)
        }
    }
    control.publish()
    report["screen_recording_after"] = CGPreflightScreenCaptureAccess()
    report["chrome_final"] = true
    control.publish()
    return status
}

if CommandLine.arguments[2] == "--owned" {
    // Without admission there is no profile or browser to clean up.
    do { exit(try runOwned(CommandLine.arguments)) }
    catch { exit(1) }
}

do {
    try save()
    try requireDeniedPreflight()
    let log = try openLog()
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
