import Foundation

@_silgen_name("thrum_mobile_start")
private func startCrowd(_ store: UnsafePointer<CChar>, _ qualification: Int32)

struct MobileApplication: ThrumMobileApplication {
    static func start() {
        let settings = ProcessInfo.processInfo.environment
        let store = settings["THRUM_ASSET_STORE"]
            ?? URL(fileURLWithPath: NSHomeDirectory()).appendingPathComponent("Documents/crowd-store").path
        store.withCString { startCrowd($0, settings["THRUM_CROWD_ROUTE"] == "1" ? 1 : 0) }
    }
}
