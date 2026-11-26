import Foundation

@_silgen_name("thrum_mobile_start")
private func startCrowd(_ store: UnsafePointer<CChar>, _ fixedSteps: Int32, _ witness: Int32)

struct MobileApplication: ThrumMobileApplication {
    static func start() {
        let settings = ProcessInfo.processInfo.environment
        let store = settings["THRUM_ASSET_STORE"]
            ?? URL(fileURLWithPath: NSHomeDirectory()).appendingPathComponent("Documents/crowd-store").path
        let fixedSteps = settings["THRUM_CROWD_ROUTE"] == "1"
        let witness = fixedSteps || settings["THRUM_CROWD_WITNESS"] == "1"
        store.withCString { startCrowd($0, fixedSteps ? 1 : 0, witness ? 1 : 0) }
    }
}
