import Foundation
import AppKit
import CoreGraphics
import CoreMedia
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

    func log(_ event: String) {
        let unixMS = UInt64(Date().timeIntervalSince1970 * 1000)
        FileHandle.standardError.write(Data("m1 recording \(event) unix_ms=\(unixMS)\n".utf8))
    }

    func recordingOutput(_ recordingOutput: SCRecordingOutput, didFailWithError error: Error) {
        FileHandle.standardError.write(Data("window recorder: \(error)\n".utf8))
        exit(2)
    }
}

private final class CompleteFrame: NSObject, SCStreamOutput {
    private let marker: URL
    private var seen = false

    init(marker: URL) {
        self.marker = marker
    }

    func stream(_ stream: SCStream, didOutputSampleBuffer sample: CMSampleBuffer,
                of type: SCStreamOutputType) {
        guard !seen, type == .screen, sample.isValid,
              let attachments = CMSampleBufferGetSampleAttachmentsArray(
                sample, createIfNecessary: false) as? [[SCStreamFrameInfo: Any]],
              let code = attachments.first?[.status] as? Int,
              SCFrameStatus(rawValue: code) == .complete
        else { return }
        let pts = CMSampleBufferGetPresentationTimeStamp(sample).seconds
        let line = "complete sample pts=\(pts)\n"
        do {
            try Data(line.utf8).write(to: marker, options: [.atomic])
            seen = true
        } catch {
            FileHandle.standardError.write(Data("complete frame marker: \(error)\n".utf8))
            exit(2)
        }
    }
}

@main
private struct WindowCapture {
    @MainActor
    static func main() async throws {
        _ = NSApplication.shared
        let arguments = CommandLine.arguments
        guard CGPreflightScreenCaptureAccess() else {
            FileHandle.standardError.write(Data("M1 SCREEN RECORDING PERMISSION REQUIRED\n".utf8))
            exit(3)
        }
        if arguments == [arguments[0], "--preflight"] { return }
        guard arguments.count == 4, let pid = Int32(arguments[1]),
              let seconds = UInt64(arguments[3]), (1...60).contains(seconds)
        else { throw CocoaError(.fileWriteInvalidFileName) }

        let output = URL(fileURLWithPath: arguments[2])
        let ready = output.appendingPathExtension("ready")
        let done = output.appendingPathExtension("done")
        guard !FileManager.default.fileExists(atPath: output.path)
        else { throw CocoaError(.fileWriteFileExists) }

        var window: SCWindow?
        var builtIn: SCDisplay?
        for _ in 0..<50 {
            let content = try await SCShareableContent.excludingDesktopWindows(
                true, onScreenWindowsOnly: true)
            let matches = content.windows.filter {
                $0.owningApplication?.processID == pid && $0.windowLayer == 0
                    && ($0.title?.hasPrefix("Thrum V1") ?? false)
            }
            guard matches.count <= 1 else { throw CocoaError(.fileReadCorruptFile) }
            if let selected = matches.first {
                let displays = content.displays.filter {
                    CGDisplayIsBuiltin($0.displayID) != 0 && $0.frame.contains(selected.frame)
                }
                guard displays.count == 1 else { throw CocoaError(.fileReadCorruptFile) }
                window = selected
                builtIn = displays.first
                break
            }
            try await Task.sleep(for: .milliseconds(100))
        }
        guard let window, let builtIn else { throw CocoaError(.fileReadNoSuchFile) }
        print("pid=\(pid) window=\(window.windowID) builtInDisplay=\(builtIn.displayID) frame=\(window.frame)")
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
        let completeFrame = CompleteFrame(marker: output.appendingPathExtension("complete"))
        try stream.addStreamOutput(completeFrame, type: .screen,
                                   sampleHandlerQueue: DispatchQueue(label: "thrum.m2.complete-frame"))
        try stream.addRecordingOutput(recording)
        try await stream.startCapture()
        try await Task.sleep(for: .seconds(seconds))
        delegate.log("stop-request")
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
