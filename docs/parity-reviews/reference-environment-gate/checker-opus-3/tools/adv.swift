import CoreText
import Foundation
let a = CommandLine.arguments
let font = CTFontCreateWithName(a[1] as CFString, CGFloat(Double(a[2])!), nil)
let s = a[3] as NSString
var chars = [UniChar](repeating: 0, count: s.length); s.getCharacters(&chars)
var glyphs = [CGGlyph](repeating: 0, count: chars.count)
CTFontGetGlyphsForCharacters(font, chars, &glyphs, chars.count)
var adv = [CGSize](repeating: .zero, count: glyphs.count)
let total = CTFontGetAdvancesForGlyphs(font, .horizontal, glyphs, &adv, glyphs.count)
print("\(a[1]) \(a[2]) '\(a[3])' advance=\(total)")
