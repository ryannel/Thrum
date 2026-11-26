import QuartzCore
import UIKit

private enum SceneBridge {
    static weak var pendingView: UIView?
    static weak var connectedScene: UIWindowScene?
    static weak var delegate: ThrumSceneDelegate?
    static var displayLink: CADisplayLink?
    static var displayTarget: DisplayTarget?
    static var detached: [WeakWindow] = []

    static func attachIfReady() {
        guard let view = pendingView,
              let scene = connectedScene,
              let window = view.window,
              let delegate else { return }
        // Winit created the controller before the window joined a scene. Reattach
        // it afterwards so UIKit updates the view's safe area when the scene rotates.
        let controller = window.rootViewController
        window.rootViewController = nil
        window.windowScene = scene
        window.rootViewController = controller
        window.makeKeyAndVisible()
        delegate.window = window
        pendingView = nil
    }
}

private final class WeakWindow {
    weak var window: UIWindow?
    init(_ window: UIWindow) { self.window = window }
}

// Poll the UIKit screen on its display tick. The render owner consumes the
// record independently of the surface lease.
private final class DisplayTarget: NSObject {
    @objc func displayFrame(_ link: CADisplayLink) {
        let screen = SceneBridge.connectedScene?.screen
        let headroom = screen?.currentEDRHeadroom ?? 1.0
        thrumIOSDisplayTarget(link.targetTimestamp, Double(headroom))
    }
}

@_silgen_name("thrum_ios_display_target")
private func thrumIOSDisplayTarget(_ targetMediaSeconds: Double, _ headroom: Double)

@_silgen_name("thrum_ios_scene_connected")
private func thrumSceneConnected()

protocol ThrumMobileApplication {
    static func start()
    static func sceneConnected(_ options: UIScene.ConnectionOptions)
    static func openURLs(_ contexts: Set<UIOpenURLContext>)
}

extension ThrumMobileApplication {
    static func sceneConnected(_ options: UIScene.ConnectionOptions) {}
    static func openURLs(_ contexts: Set<UIOpenURLContext>) {}
}

@_cdecl("thrum_ios_safe_area")
func thrumIOSSafeArea(_ rawView: UnsafeMutableRawPointer, _ insets: UnsafeMutablePointer<Double>) {
    let view = Unmanaged<UIView>.fromOpaque(rawView).takeUnretainedValue()
    let area = view.safeAreaInsets
    insets[0] = Double(area.left)
    insets[1] = Double(area.top)
    insets[2] = Double(area.right)
    insets[3] = Double(area.bottom)
}



@_cdecl("thrum_ios_start_display_link")
func thrumIOSStartDisplayLink() -> Int32 {
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

@_cdecl("thrum_ios_stop_display_link")
func thrumIOSStopDisplayLink() {
    SceneBridge.displayLink?.invalidate()
    SceneBridge.displayLink = nil
    SceneBridge.displayTarget = nil
}

@objc(ThrumSceneDelegate)
final class ThrumSceneDelegate: UIResponder, UIWindowSceneDelegate {
    var window: UIWindow?

    func scene(_ scene: UIScene, willConnectTo session: UISceneSession,
               options connectionOptions: UIScene.ConnectionOptions) {
        guard let connected = scene as? UIWindowScene else { return }
        SceneBridge.connectedScene = connected
        SceneBridge.delegate = self
        SceneBridge.attachIfReady()
        MobileApplication.sceneConnected(connectionOptions)
        thrumSceneConnected()
    }

    func scene(_ scene: UIScene, openURLContexts contexts: Set<UIOpenURLContext>) {
        MobileApplication.openURLs(contexts)
    }

    func sceneDidDisconnect(_ scene: UIScene) {
        thrumIOSStopDisplayLink()
        window = nil
        SceneBridge.connectedScene = nil
        SceneBridge.delegate = nil
    }
}

// Winit 0.30 creates its UIWindow using init(frame:). The Rust shell passes
// Winit's live UIView synchronously; UIKit attaches its owning window to the
// active scene. The pointer is borrowed only during this call.
@_cdecl("thrum_ios_attach_view")
func thrumIOSAttachView(_ rawView: UnsafeMutableRawPointer) {
    let view = Unmanaged<UIView>.fromOpaque(rawView).takeUnretainedValue()
    SceneBridge.pendingView = view
    SceneBridge.attachIfReady()
}

// Keep the drawable's aspect while UIKit animates the view's bounds through a
// rotation. raw-window-metal leaves its sublayer on kCAGravityResize, which
// squeezes a landscape drawable into the still-portrait animating bounds. At
// rest the drawable matches the bounds, so this changes nothing there.
@_cdecl("thrum_ios_fit_layer")
func thrumIOSFitLayer(_ rawView: UnsafeMutableRawPointer) -> Int32 {
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
@_cdecl("thrum_ios_detach_view")
func thrumIOSDetachView(_ rawView: UnsafeMutableRawPointer) {
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

@_cdecl("thrum_ios_live_detached_windows")
func thrumIOSLiveDetachedWindows() -> Int32 {
    SceneBridge.detached.removeAll { $0.window == nil }
    return Int32(SceneBridge.detached.count)
}

MobileApplication.start()
