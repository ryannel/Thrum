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

    /// Drag right 10 pt (one eighth deflection) and hold half a second: the
    /// controlled actor, the grid's top-right square, steps just past the
    /// grid's right edge, where it stands alone against the sky.
    func testNudge() {
        let (_, origin, _) = start()
        let end = origin.withOffset(CGVector(dx: 10, dy: 0))
        origin.press(forDuration: 0.1, thenDragTo: end, withVelocity: .slow, thenHoldForDuration: 0.5)
    }

    /// Drag up 200 pt and hold thirty seconds while the route backgrounds the
    /// app or replaces its surface, then release. Posting a route command can
    /// take many seconds on a loaded host; the hold must outlast it.
    func testHold() {
        continueAfterFailure = true
        let (_, origin, end) = start()
        origin.press(forDuration: 0.1, thenDragTo: end, withVelocity: .slow, thenHoldForDuration: 30)
    }

    /// Rotate the device to landscape (home indicator on the right).
    func testLandscape() {
        XCUIDevice.shared.orientation = .landscapeLeft
        sleep(2)
    }

    /// Rotate the device back to portrait.
    func testPortrait() {
        XCUIDevice.shared.orientation = .portrait
        sleep(2)
    }
}
