import CoreText
import Foundation
// For each hex code point argument: primary glyph?, CTFontCreateForString fallback name + path + glyph.
let primary = CTFontCreateWithName(CommandLine.arguments[1] as CFString, 2048, nil)
for a in CommandLine.arguments.dropFirst(2) {
    let cp = UInt32(a, radix: 16)!
    let s = String(Character(Unicode.Scalar(cp)!)); var u = Array(s.utf16); var g = [CGGlyph](repeating: 0, count: u.count)
    let own = CTFontGetGlyphsForCharacters(primary, &u, &g, u.count) && g[0] != 0
    let fb = CTFontCreateForString(primary, s as CFString, CFRange(location: 0, length: u.count))
    var g2 = [CGGlyph](repeating: 0, count: u.count)
    let has = CTFontGetGlyphsForCharacters(fb, &u, &g2, u.count) && g2[0] != 0
    let p = (CTFontCopyAttribute(fb, kCTFontURLAttribute) as? URL)?.path ?? "-"
    print("U+\(a)\town=\(own)\tfb=\(CTFontCopyPostScriptName(fb) as String)\tglyph=\(has)\t\(p)")
}
