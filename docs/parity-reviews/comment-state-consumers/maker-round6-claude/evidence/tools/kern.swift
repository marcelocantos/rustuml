import CoreText
import Foundation
// kern table subtables of each font: format and coverage (cross-stream: OT bit 0x04, Apple bit 0x4000).
for n in CommandLine.arguments.dropFirst() {
    let f = CTFontCreateWithName(n as CFString, 2048, nil)
    let tag = "kern".utf8.reduce(UInt32(0)) { $0 << 8 | UInt32($1) }
    guard let d = CTFontCopyTable(f, CTFontTableTag(tag), CTFontTableOptions(rawValue: 0)) else { print("\(n)\tno kern"); continue }
    let b = [UInt8](d as Data)
    func u16(_ o: Int) -> Int { Int(b[o]) << 8 | Int(b[o + 1]) }
    func u32(_ o: Int) -> Int { u16(o) << 16 | u16(o + 2) }
    var out: [String] = []
    if u16(0) == 0 { var o = 4; for _ in 0..<u16(2) { let cov = u16(o + 4); out.append("ot fmt=\(cov >> 8) cross=\(cov & 0x04 != 0)"); o += u16(o + 2) } }
    else { var o = 8; for _ in 0..<u32(4) { let cov = u16(o + 4); out.append("apple fmt=\(cov & 0xFF) cross=\(cov & 0x4000 != 0)"); o += u32(o) } }
    print("\(n)\t\(out.joined(separator: "; "))")
}
