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

    /// Tap the centre of a button in the app's submitted layout.
    func testInterfaceTap() throws {
        let point = try XCTUnwrap(ProcessInfo.processInfo.environment["THRUM_INTERFACE_TEST_POINT"])
            .split(separator: ",").compactMap { Double($0) }
        XCTAssertEqual(point.count, 2)
        guard point.count == 2 else { return }
        let app = XCUIApplication(bundleIdentifier: "org.thrum.v1")
        app.coordinate(withNormalizedOffset: CGVector(dx: point[0], dy: point[1])).tap()
    }

    /// Open the app through UIKit with a real invite URL.
    func testInterfaceInvite() throws {
        let environment = ProcessInfo.processInfo.environment
        let address = try XCTUnwrap(environment["THRUM_INTERFACE_TEST_URL"])
        let url = try XCTUnwrap(URL(string: address))
        let app = XCUIApplication(bundleIdentifier: "org.thrum.v1")
        app.launchEnvironment = [
            "THRUM_V1_ROUTE": "1",
            "THRUM_SCENE": "interface",
            "THRUM_INTERFACE_TEXT": "menu",
            "THRUM_FONT_STORE": try XCTUnwrap(environment["THRUM_INTERFACE_TEST_STORE"]),
        ]
        app.open(url)
    }

    /// Return the existing app to the foreground after another UIKit scene took focus.
    func testInterfaceForeground() {
        XCUIApplication(bundleIdentifier: "org.thrum.v1").activate()
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
