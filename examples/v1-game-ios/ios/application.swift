import UIKit

#if THRUM_PROVING
@_silgen_name("thrum_ios_shell_command")
private func thrumIOSCommand(_ code: Int32)

@_silgen_name("thrum_v1_open_invite")
private func thrumV1OpenInvite(_ bytes: UnsafePointer<UInt8>?, _ count: Int,
                              _ fontStore: UnsafePointer<CChar>?)

private func inviteTicket(_ url: URL) -> String? {
    guard let components = URLComponents(url: url, resolvingAgainstBaseURL: false),
          components.scheme == "thrum-v1", components.host == "join",
          components.query == nil, components.fragment == nil,
          let ticket = components.percentEncodedPath.dropFirst().removingPercentEncoding,
          !ticket.isEmpty else { return nil }
    return ticket
}

private func forwardInvite(_ ticket: String) {
    let store = ProcessInfo.processInfo.environment["THRUM_FONT_STORE"]
        ?? URL(fileURLWithPath: NSHomeDirectory()).appendingPathComponent("Documents/interface-store").path
    Array(ticket.utf8).withUnsafeBufferPointer { buffer in
        store.withCString { thrumV1OpenInvite(buffer.baseAddress, buffer.count, $0) }
    }
}

// Qualification commands arrive as Darwin notifications on the main run loop
// (`notifyutil -p org.thrum.v1.<command>` inside the simulator). Opened URLs
// would ask the user to confirm. The Rust shell honours them only when the app
// was launched with THRUM_V1_ROUTE=1.
private let shellCommandPrefix = "org.thrum.v1."
private let shellCommands: [String: Int32] = [
    "replace-surface": 1,
    "hide-view": 2,
    "show-view": 3,
    "stall-render": 4,
    "stall-present": 5,
    "sample-actor": 6,
    "lose-device": 7,
    "hold-recover": 8,
    "release-recover": 9,
    "stale-wake": 10,
    "cooked-pan": 11,
    "capture-begin": 12,
    "capture-end": 13,
    "capture-saturate": 14,
    "capture-slow": 15,
    "capture-move": 16,
    "capture-rest": 17,
    "force-headroom": 18,
    "hold-world": 19,
    "run-world": 20,
    "capture-still": 21,
    "open-choice": 22,
    "close-choice": 23,
]

// Route commands also arrive as files dropped into the app's temporary
// directory: on a heavily loaded host each `simctl spawn notifyutil` can take
// over a minute, while the route can write the container directly. A dispatch
// source on the directory runs on the main queue; files run in name order.
private var observingShellCommands = false
private var commandDirectorySource: DispatchSourceFileSystemObject?

private func observeShellCommands() {
    // Scene reconnects must not register a second set of observers.
    guard !observingShellCommands else { return }
    observingShellCommands = true
    let center = CFNotificationCenterGetDarwinNotifyCenter()
    for command in shellCommands.keys {
        CFNotificationCenterAddObserver(center, nil, { _, _, name, _, _ in
            guard let name = name?.rawValue as String?,
                  let code = shellCommands[String(name.dropFirst(shellCommandPrefix.count))]
            else { return }
            thrumIOSCommand(code)
        }, (shellCommandPrefix + command) as CFString, nil, .deliverImmediately)
    }
    guard ProcessInfo.processInfo.environment["THRUM_V1_ROUTE"] == "1" else { return }
    let directory = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("thrum-v1-commands")
    try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    let descriptor = open(directory.path, O_EVTONLY)
    guard descriptor >= 0 else { return }
    let source = DispatchSource.makeFileSystemObjectSource(fileDescriptor: descriptor, eventMask: .write, queue: .main)
    source.setEventHandler {
        let names = (try? FileManager.default.contentsOfDirectory(atPath: directory.path)) ?? []
        for name in names.sorted() where !name.hasPrefix(".") {
            try? FileManager.default.removeItem(at: directory.appendingPathComponent(name))
            // "<sequence>-<command>"
            guard let dash = name.firstIndex(of: "-"),
                  let code = shellCommands[String(name[name.index(after: dash)...])]
            else { continue }
            thrumIOSCommand(code)
        }
    }
    source.setCancelHandler { close(descriptor) }
    source.resume()
    commandDirectorySource = source
}

#endif

struct MobileApplication: ThrumMobileApplication {
    #if THRUM_PROVING
    static func sceneConnected(_ options: UIScene.ConnectionOptions) {
        observeShellCommands()
        if let ticket = options.urlContexts.lazy.compactMap({ inviteTicket($0.url) }).first {
            forwardInvite(ticket)
        }
    }

    static func openURLs(_ contexts: Set<UIOpenURLContext>) {
        for context in contexts {
            if let ticket = inviteTicket(context.url) { forwardInvite(ticket) }
        }
    }
    #endif
}
