import XCTest

// Real HID touches for the G3b simulator route. The route launches the game
// itself; these tests attach to the running process and never relaunch it.
// Each gesture starts mid-screen and drags up, which the game reads as forward.
final class ThrumV1Touch: XCTestCase {
    private func start() -> (XCUIApplication, XCUICoordinate, XCUICoordinate) {
        let app = XCUIApplication(bundleIdentifier: "org.thrum.v1")
        let origin = app.coordinate(withNormalizedOffset: CGVector(dx: 0.3, dy: 0.7))
        let end = origin.withOffset(CGVector(dx: 0, dy: -200))
        return (app, origin, end)
    }

    /// Drag up 200 pt, hold one second, release.
    func testSwipe() {
        let (_, origin, end) = start()
        origin.press(forDuration: 0.1, thenDragTo: end, withVelocity: .slow, thenHoldForDuration: 1)
    }

    /// Drag right 200 pt and hold three seconds: the controlled actor drives
    /// sideways far enough to leave the grid visibly, then stops on release.
    func testDrive() {
        let (_, origin, _) = start()
        let end = origin.withOffset(CGVector(dx: 150, dy: 0))
        origin.press(forDuration: 0.1, thenDragTo: end, withVelocity: .slow, thenHoldForDuration: 3)
    }

    /// Drag up 200 pt and hold ten seconds while the route backgrounds the app
    /// or replaces its surface, then release.
    func testHold() {
        continueAfterFailure = true
        let (_, origin, end) = start()
        origin.press(forDuration: 0.1, thenDragTo: end, withVelocity: .slow, thenHoldForDuration: 10)
    }
}
