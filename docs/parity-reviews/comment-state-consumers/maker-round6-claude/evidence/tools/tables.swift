import CoreText
import Foundation
// For each font name: PostScript name, path, and the sfnt table tags CoreText exposes
// (CTFontCopyAvailableTables stores each CTFontTableTag as the array value pointer itself).
for n in CommandLine.arguments.dropFirst() {
    let f = CTFontCreateWithName(n as CFString, 2048, nil)
    var tags: [String] = []
    if let arr = CTFontCopyAvailableTables(f, CTFontTableOptions(rawValue: 0)) {
        for i in 0..<CFArrayGetCount(arr) {
            let v = UInt32(truncatingIfNeeded: UInt(bitPattern: CFArrayGetValueAtIndex(arr, i)))
            tags.append(String(bytes: [UInt8(v >> 24 & 0xff), UInt8(v >> 16 & 0xff), UInt8(v >> 8 & 0xff), UInt8(v & 0xff)], encoding: .ascii) ?? "?")
        }
    }
    let p = (CTFontCopyAttribute(f, kCTFontURLAttribute) as? URL)?.path ?? "-"
    print("\(n)\t\(CTFontCopyPostScriptName(f) as String)\t\(p)\t\(tags.sorted().joined(separator: ","))")
}
