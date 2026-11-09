import QuartzCore
import UIKit

private enum SceneBridge {
    static weak var pendingView: UIView?
    static weak var connectedScene: UIWindowScene?
    static weak var delegate: ThrumV1SceneDelegate?
    static var displayLink: CADisplayLink?
    static var displayTarget: DisplayTarget?
    static var detached: [WeakWindow] = []

    static func attachIfReady() {
        guard let view = pendingView,
              let scene = connectedScene,
              let window = view.window,
              let delegate else { return }
        window.windowScene = scene
        window.makeKeyAndVisible()
        delegate.window = window
        pendingView = nil
    }
}

private final class WeakWindow {
    weak var window: UIWindow?
    init(_ window: UIWindow) { self.window = window }
}

// The link only forwards its media-clock target; the render owner converts it
// to a presentation lead when it consumes the request.
private final class DisplayTarget: NSObject {
    @objc func displayFrame(_ link: CADisplayLink) {
        thrumV1DisplayTarget(link.targetTimestamp)
    }
}

@_silgen_name("thrum_v1_display_target")
private func thrumV1DisplayTarget(_ targetMediaSeconds: Double)

@_silgen_name("thrum_v1_shell_command")
private func thrumV1ShellCommand(_ code: Int32)

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
            thrumV1ShellCommand(code)
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
            thrumV1ShellCommand(code)
        }
    }
    source.setCancelHandler { close(descriptor) }
    source.resume()
    commandDirectorySource = source
}

@_cdecl("thrum_v1_start_display_link")
func thrumV1StartDisplayLink() -> Int32 {
    guard SceneBridge.connectedScene != nil else { return 0 }
    if SceneBridge.displayLink != nil { return 1 }
    let target = DisplayTarget()
    let link = CADisplayLink(target: target, selector: #selector(DisplayTarget.displayFrame(_:)))
    link.preferredFrameRateRange = CAFrameRateRange(minimum: 60, maximum: 120, preferred: 120)
    link.add(to: .main, forMode: .common)
    SceneBridge.displayTarget = target
    SceneBridge.displayLink = link
    return 1
}

@_cdecl("thrum_v1_stop_display_link")
func thrumV1StopDisplayLink() {
    SceneBridge.displayLink?.invalidate()
    SceneBridge.displayLink = nil
    SceneBridge.displayTarget = nil
}

@objc(ThrumV1SceneDelegate)
final class ThrumV1SceneDelegate: UIResponder, UIWindowSceneDelegate {
    var window: UIWindow?

    func scene(_ scene: UIScene, willConnectTo session: UISceneSession,
               options connectionOptions: UIScene.ConnectionOptions) {
        guard let connected = scene as? UIWindowScene else { return }
        SceneBridge.connectedScene = connected
        SceneBridge.delegate = self
        SceneBridge.attachIfReady()
        observeShellCommands()
    }

    func sceneDidDisconnect(_ scene: UIScene) {
        thrumV1StopDisplayLink()
        window = nil
        SceneBridge.connectedScene = nil
        SceneBridge.delegate = nil
    }
}

// Winit 0.30 creates its UIWindow using init(frame:). The Rust shell passes
// Winit's live UIView synchronously; UIKit attaches its owning window to the
// active scene. The pointer is borrowed only during this call.
@_cdecl("thrum_v1_attach_view")
func thrumV1AttachView(_ rawView: UnsafeMutableRawPointer) {
    let view = Unmanaged<UIView>.fromOpaque(rawView).takeUnretainedValue()
    SceneBridge.pendingView = view
    SceneBridge.attachIfReady()
}

// Keep the drawable's aspect while UIKit animates the view's bounds through a
// rotation. raw-window-metal leaves its sublayer on kCAGravityResize, which
// squeezes a landscape drawable into the still-portrait animating bounds. At
// rest the drawable matches the bounds, so this changes nothing there.
@_cdecl("thrum_v1_fit_layer")
func thrumV1FitLayer(_ rawView: UnsafeMutableRawPointer) -> Int32 {
    let view = Unmanaged<UIView>.fromOpaque(rawView).takeUnretainedValue()
    var count: Int32 = 0
    for layer in view.layer.sublayers ?? [] where layer is CAMetalLayer {
        layer.contentsGravity = .resizeAspect
        count += 1
    }
    return count
}

// Close a window whose surface the Rust shell released. Winit does not close
// its UIWindow on drop and raw-window-metal never removes the CAMetalLayer
// sublayer it added, so both would otherwise outlive the surface. Only a weak
// reference is kept, to witness that the window deallocates.
@_cdecl("thrum_v1_detach_view")
func thrumV1DetachView(_ rawView: UnsafeMutableRawPointer) {
    let view = Unmanaged<UIView>.fromOpaque(rawView).takeUnretainedValue()
    if SceneBridge.pendingView === view { SceneBridge.pendingView = nil }
    for layer in view.layer.sublayers ?? [] where layer is CAMetalLayer {
        layer.removeFromSuperlayer()
    }
    guard let window = view.window else { return }
    window.isHidden = true
    if SceneBridge.delegate?.window === window { SceneBridge.delegate?.window = nil }
    window.windowScene = nil
    SceneBridge.detached.removeAll { $0.window == nil }
    SceneBridge.detached.append(WeakWindow(window))
}

@_cdecl("thrum_v1_live_detached_windows")
func thrumV1LiveDetachedWindows() -> Int32 {
    SceneBridge.detached.removeAll { $0.window == nil }
    return Int32(SceneBridge.detached.count)
}

@_silgen_name("thrum_v1_mobile_start")
func thrumV1MobileStart()

thrumV1MobileStart()
