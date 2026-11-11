import CoreGraphics
import Foundation
let all=CGWindowListCopyWindowInfo([.optionOnScreenOnly,.excludeDesktopElements],kCGNullWindowID) as? [[String:Any]] ?? []
let pid=CommandLine.arguments.count>1 ? Int(CommandLine.arguments[1]) : nil
let selected=all.filter { pid == nil || ($0[kCGWindowOwnerPID as String] as? Int)==pid }
let data=try JSONSerialization.data(withJSONObject:selected,options:[.prettyPrinted,.sortedKeys])
FileHandle.standardOutput.write(data)
