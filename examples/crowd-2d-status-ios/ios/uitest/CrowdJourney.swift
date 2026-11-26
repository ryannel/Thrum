import XCTest

final class CrowdJourney: XCTestCase {
    func testDragAndRotate() {
        let app = XCUIApplication(bundleIdentifier: "org.thrum.crowd")
        XCTAssertEqual(app.state, .runningForeground)
        let start = app.coordinate(withNormalizedOffset: CGVector(dx: 0.3, dy: 0.7))
        let end = start.withOffset(CGVector(dx: 115, dy: -65))
        start.press(forDuration: 0.1, thenDragTo: end, withVelocity: .slow, thenHoldForDuration: 0)
        XCUIDevice.shared.orientation = .landscapeLeft
    }

    func testReturnToForeground() {
        let app = XCUIApplication(bundleIdentifier: "org.thrum.crowd")
        app.activate()
        XCTAssertEqual(app.state, .runningForeground)
    }

    func testPortrait() {
        XCUIDevice.shared.orientation = .portrait
    }
}
