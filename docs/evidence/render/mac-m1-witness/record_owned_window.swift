import Foundation
import ScreenCaptureKit

private final class Recorder: NSObject, SCRecordingOutputDelegate {
    let ready: URL
    let done: URL

    init(ready: URL, done: URL) {
        self.ready = ready
        self.done = done
    }

    func recordingOutputDidStartRecording(_ recordingOutput: SCRecordingOutput) {
        log("start")
        FileManager.default.createFile(atPath: ready.path, contents: Data())
    }

    func recordingOutputDidFinishRecording(_ recordingOutput: SCRecordingOutput) {
        log("finish")
        FileManager.default.createFile(atPath: done.path, contents: Data())
    }

    private func log(_ event: String) {
        let unixMS = UInt64(Date().timeIntervalSince1970 * 1000)
        FileHandle.standardError.write(Data("m1 recording \(event) unix_ms=\(unixMS)\n".utf8))
    }

    func recordingOutput(_ recordingOutput: SCRecordingOutput, didFailWithError error: Error) {
        FileHandle.standardError.write(Data("window recorder: \(error)\n".utf8))
        exit(2)
    }
}

@main
private struct WindowCapture {
    static func main() async throws {
        let arguments = CommandLine.arguments
        guard arguments.count == 4, let pid = Int32(arguments[1]),
              let seconds = UInt64(arguments[3]), (1...60).contains(seconds)
        else { throw CocoaError(.fileWriteInvalidFileName) }

        let output = URL(fileURLWithPath: arguments[2])
        let ready = output.appendingPathExtension("ready")
        let done = output.appendingPathExtension("done")
        guard !FileManager.default.fileExists(atPath: output.path)
        else { throw CocoaError(.fileWriteFileExists) }

        var window: SCWindow?
        for _ in 0..<50 {
            let content = try await SCShareableContent.excludingDesktopWindows(
                true, onScreenWindowsOnly: true)
            window = content.windows.first {
                $0.owningApplication?.processID == pid && $0.windowLayer == 0
            }
            if window != nil { break }
            try await Task.sleep(for: .milliseconds(100))
        }
        guard let window else { throw CocoaError(.fileReadNoSuchFile) }
        print("pid=\(pid) window=\(window.windowID) frame=\(window.frame)")
        let filter = SCContentFilter(desktopIndependentWindow: window)
        let config = SCStreamConfiguration()
        config.width = 1280
        config.height = Int((1280 * window.frame.height / window.frame.width) / 2) * 2
        config.minimumFrameInterval = CMTime(value: 1, timescale: 60)
        config.showsCursor = false
        let stream = SCStream(filter: filter, configuration: config, delegate: nil)
        let recordingConfig = SCRecordingOutputConfiguration()
        recordingConfig.outputURL = output
        recordingConfig.outputFileType = .mp4
        let delegate = Recorder(ready: ready, done: done)
        let recording = SCRecordingOutput(configuration: recordingConfig, delegate: delegate)
        try stream.addRecordingOutput(recording)
        try await stream.startCapture()
        try await Task.sleep(for: .seconds(seconds))
        try stream.removeRecordingOutput(recording)
        for _ in 0..<50 {
            if FileManager.default.fileExists(atPath: done.path) {
                try await stream.stopCapture()
                return
            }
            try await Task.sleep(for: .milliseconds(100))
        }
        try await stream.stopCapture()
        throw CocoaError(.fileWriteUnknown)
    }
}
