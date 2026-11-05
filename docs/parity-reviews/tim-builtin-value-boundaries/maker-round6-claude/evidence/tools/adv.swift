import CoreText
import Foundation
// For each hex code point: the primary font's own glyph id and its advance in font units (2048 em).
let f = CTFontCreateWithName(CommandLine.arguments[1] as CFString, 2048, nil)
for a in CommandLine.arguments.dropFirst(2) {
    var u = Array(String(Character(Unicode.Scalar(UInt32(a, radix: 16)!)!)).utf16)
    var g = [CGGlyph](repeating: 0, count: u.count); var adv = [CGSize](repeating: .zero, count: 1)
    let ok = CTFontGetGlyphsForCharacters(f, &u, &g, u.count)
    CTFontGetAdvancesForGlyphs(f, .horizontal, &g, &adv, 1)
    print("U+\(a)\tmapped=\(ok)\tglyph=\(g[0])\tadvance=\(adv[0].width)")
}
