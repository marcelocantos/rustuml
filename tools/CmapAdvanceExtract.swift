// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
//
// Environment extraction (not Java): the own-cmap advances of the face JDK 21 measures for a
// (family, style) slot. Face resolution is tools/PrimaryFaceExtract.swift (AWTFont.m
// awtFontForName). A scalar is included only when CTFontGetGlyphsForCharacters returns the
// face's own glyph (JDK CCompositeGlyphMapper slot 0). The advance is
// CTFontGetAdvancesForGlyphs at size 2048, in font units, summed over the non-zero glyphs of
// that scalar. Glyph 0 is not measured, so a padding slot does not add the .notdef advance.
//
// Usage:
//   swiftc -O tools/CmapAdvanceExtract.swift -o "$OUT/cmap-advance-extract"
//   "$OUT/cmap-advance-extract" Verdana-Italic 2 > Verdana-Italic.italic.cmap.tsv
// Output: header lines, then
// "M<TAB>code (hex)<TAB>glyph id[,glyph id]<TAB>advance (font units)"
// for every scalar the face maps, in code-point order. U+0020..U+007E are included.
import AppKit
import CoreText
import Foundation

let name = CommandLine.arguments[1]
let style = Int(CommandLine.arguments[2])!
guard var nsFont = NSFont(name: name, size: 2048) else {
    FileHandle.standardError.write("no NSFont named \(name)\n".data(using: .utf8)!)
    exit(2)
}
if style & 2 != 0 { nsFont = NSFontManager.shared.convert(nsFont, toHaveTrait: .italicFontMask) }
if style & 1 != 0 { nsFont = NSFontManager.shared.convert(nsFont, toHaveTrait: .boldFontMask) }
let face = nsFont as CTFont
let ps = CTFontCopyPostScriptName(face) as String
let path = (CTFontCopyAttribute(face, kCTFontURLAttribute) as? URL)?.path ?? "-"
print("#host\t\(ProcessInfo.processInfo.operatingSystemVersionString)")
print("#java_face\t\(name)\t\(style)\t\(ps)")
print("#file\t\(path)")
print("#units_per_em\t\(CTFontGetUnitsPerEm(face))\tsize\t\(CTFontGetSize(face))")

var mapped = 0
for cp in UInt32(0)...UInt32(0x10FFFF) {
    if cp >= 0xD800 && cp <= 0xDFFF { continue }
    var units = Array(String(Character(Unicode.Scalar(cp)!)).utf16)
    var glyphs = [CGGlyph](repeating: 0, count: units.count)
    let own = CTFontGetGlyphsForCharacters(face, &units, &glyphs, units.count) && glyphs.contains { $0 != 0 }
    if !own { continue }
    var width: CGFloat = 0
    var ids: [String] = []
    for glyph in glyphs where glyph != 0 {
        var one = [glyph]
        var adv = [CGSize](repeating: .zero, count: 1)
        CTFontGetAdvancesForGlyphs(face, .horizontal, &one, &adv, 1)
        width += adv[0].width
        ids.append(String(glyph))
    }
    print("M\t\(String(cp, radix: 16, uppercase: true))\t\(ids.joined(separator: ","))\t\(width)")
    mapped += 1
}
print("#mapped\t\(mapped)")
