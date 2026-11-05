// For each PostScript font name and code point: does the font's own cmap map it (glyph id, advance in units)?
import CoreText
import Foundation
let args = CommandLine.arguments.dropFirst()
let names = args.first!.split(separator: ",").map(String.init)
let codes = args.dropFirst().map { UInt32($0, radix: 16)! }
for n in names {
  let f = CTFontCreateWithName(n as CFString, 2048, nil)
  let actual = CTFontCopyPostScriptName(f) as String
  for c in codes {
    var u = Array(String(UnicodeScalar(c)!).utf16); var g = [CGGlyph](repeating: 0, count: u.count)
    let ok = CTFontGetGlyphsForCharacters(f, &u, &g, u.count)
    var adv = CGSize.zero; var gg = g[0]
    CTFontGetAdvancesForGlyphs(f, .horizontal, &gg, &adv, 1)
    print("\(n)\t\(actual)\t\(String(c, radix: 16, uppercase: true))\t\(ok ? 1 : 0)\t\(g[0])\t\(adv.width)")
  }
}
