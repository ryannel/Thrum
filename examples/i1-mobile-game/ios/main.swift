import UIKit

// UIKit 27 requires a scene delegate even for this single-window shell.
// Winit still owns the window and game event loop.
@objc(ThrumSceneDelegate)
final class ThrumSceneDelegate: UIResponder, UIWindowSceneDelegate {}

@_silgen_name("thrum_i1_mobile_start")
func thrumI1MobileStart()

thrumI1MobileStart()
