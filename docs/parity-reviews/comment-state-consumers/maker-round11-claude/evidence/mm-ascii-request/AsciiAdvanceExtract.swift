// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
//
// Environment extraction (not Java): the printable-ASCII advances of the face JDK 21 measures for a
// (family, style) slot.
//
//  face      resolved exactly as tools/PrimaryFaceExtract.swift resolves it (AWTFont.m awtFontForName
//            71-111: [NSFont fontWithName:size:], then convertFont:toHaveTrait: italic, then bold); a
//            derived italic (CFont.createItalicVariant 177-185) keeps the upright face. "#java_face"
//            prints the resolved PostScript name, "#file" its font file.
//  advance   at size 2048 the CoreText advance of a glyph is in font units (AWTFont.m 92-111 opens the
//            NSFont at the requested size; CTFontGetAdvancesForGlyphs is linear in the size). JDK 21
//            CStrike.getGlyphAdvance -> CGGlyphImages / CTFontGetAdvancesForGlyphs gives the same
//            design advance scaled by the point size under fractional metrics, which PlantUML's
//            StringBounder uses (Font.getStringBounds = sum of advances for simple Latin text).
//            The glyph is the face's own glyph (CTFontGetGlyphsForCharacters), as in
//            CCompositeGlyphMapper slot 0.
//
// Usage:
//   swiftc -O tools/AsciiAdvanceExtract.swift -o /tmp/ascii-advance-extract
//   /tmp/ascii-advance-extract Verdana-Italic 2 > Verdana-Italic.italic.ascii.tsv
// Output: header lines, then "A<TAB>code (hex)<TAB>glyph id<TAB>advance (font units)" for U+0020..U+007E;
// a code point the face does not map prints glyph 0 and the .notdef advance.
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
for cp in UInt16(0x20)...UInt16(0x7E) {
    var units: [UniChar] = [cp]
    var glyphs: [CGGlyph] = [0]
    if !CTFontGetGlyphsForCharacters(face, &units, &glyphs, 1) { glyphs[0] = 0 }
    var adv = [CGSize](repeating: .zero, count: 1)
    CTFontGetAdvancesForGlyphs(face, .horizontal, &glyphs, &adv, 1)
    print("A\t\(String(cp, radix: 16, uppercase: true))\t\(glyphs[0])\t\(adv[0].width)")
}
