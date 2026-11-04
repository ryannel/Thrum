import UIKit

private enum SceneBridge {
    static weak var pendingView: UIView?
    static weak var connectedScene: UIWindowScene?
    static weak var delegate: ThrumV1SceneDelegate?

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
