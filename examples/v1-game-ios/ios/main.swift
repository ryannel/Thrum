import UIKit

private enum SceneBridge {
    static weak var pendingView: UIView?
    static weak var connectedScene: UIWindowScene?
    static weak var delegate: ThrumV1SceneDelegate?
    static var displayLink: CADisplayLink?
    static var displayTarget: DisplayTarget?

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

private final class DisplayTarget: NSObject {
    @objc func displayFrame(_ link: CADisplayLink) {
        thrumV1DisplayTarget(max(0, link.targetTimestamp - CACurrentMediaTime()))
    }
}

@_silgen_name("thrum_v1_display_target")
private func thrumV1DisplayTarget(_ leadSeconds: Double)

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

@_silgen_name("thrum_v1_mobile_start")
func thrumV1MobileStart()

thrumV1MobileStart()
