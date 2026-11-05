// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
//
// Environment extraction (not Java): per face JDK 21 measures for a (family, style) slot, the
// scalars its own cmap maps and the sfnt facts HarfBuzz 15.0.0's planner reads.
//
//  face      JDK 21.0.5 measures the CFont of the family's style slot (CFontManager.loadFonts 209-235,
//            registerItalicDerived 174-203, Font2D.setStyle 151-175, FontFamily.getFont 320-360), whose
//            native font is CFont.createNativeFont(nativeFontName, style) (CFont 187-191) -> AWTFont.m
//            awtFontForName 71-111: [NSFont fontWithName:], then convertFont:toHaveTrait: italic, then
//            bold. A derived italic (CFont.createItalicVariant 177-185) keeps the upright face's name.
//            Arguments: the slot's nativeFontName and Java style (0 plain, 1 bold, 2 italic, 3 both);
//            "#java_face" prints the resolved PostScript name and file.
//  own cmap  JDK 21 CCompositeGlyphMapper.convertToGlyph (54-67) puts a character in slot 0 (the
//            CFont itself) when slot 0's CCharToGlyphMapper returns a code > 0, which
//            CoreTextSupport.m CTS_GetGlyphsAsIntsForCharacters (99-135) does only for the font's
//            own glyph (CTFontGetGlyphsForCharacters); a cascade substitution is negative. A
//            character no slot maps is missingGlyph, also slot 0. FontRunIterator.next splits the
//            text at every slot change and GlyphLayout.layout (408-431) shapes one HarfBuzz buffer
//            per script run and slot run.
//  hb_plan   as tools/FontChainExtract.swift: GDEF glyph classes and the class of glyph 0, GSUB, GPOS,
//            morx, kerx, kern and its subtable formats (format 1 is a state machine).
//  cross_kern  hb_ot_layout_has_cross_kerning (kern subtable coverage CrossStream: OpenType
//            bit 0x04, Apple bit 0x4000), read by hb-ot-shape.cc 198-203 (15.0.0).
//  trak, STAT  hb-ot-shape.cc 218 (15.0.0): trak applies by default when both are present.
//  gsub_scripts, gpos_scripts  the ScriptList tags hb_ot_layout_table_select_script (hb-ot-layout.cc,
//            15.0.0) chooses from; the hebrew shaper's gpos_tag 'hebr' disables GPOS when the GPOS
//            table's chosen script is not 'hebr' (hb-ot-shape.cc 133-134, hb-ot-shaper-hebrew.cc).
//  gdef_length, gsub_length, gpos_length  the CTFontCopyTable byte counts (0 when absent): JDK's
//            HarfBuzz face reads tables through hb-jdk-font.cc reference_table -> CFont.getTableBytes
//            -> AWTFont.m Java_sun_font_CFont_getTableBytesNative (CTFontCopyTable, no options), and
//            OT::GDEF::is_blocklisted (hb-ot-layout.cc 152-266, 15.0.0) keys on these three lengths.
//
// Usage:
//   swiftc -O tools/PrimaryFaceExtract.swift -o /tmp/primary-face-extract
//   /tmp/primary-face-extract CourierNewPS-ItalicMT 2 > CourierNewPS-ItalicMT.italic.own.tsv
// Output: header lines, then "P<TAB>start<TAB>end" (runs the face's own cmap maps, all planes).
import AppKit
import CoreText
import Foundation

let name = CommandLine.arguments[1]
let style = Int(CommandLine.arguments[2])!
// AWTFont.m awtFontForName 92-111 (size 2048 here: advances in font units).
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
print("#face\t\(name)\t\(ps)\t\(path)")
print("#java_face\t\(name)\t\(style)\t\(ps)")
var g0: [CGGlyph] = [0]; var adv = [CGSize](repeating: .zero, count: 1)
CTFontGetAdvancesForGlyphs(face, .horizontal, &g0, &adv, 1)
print("#notdef_advance_units\t\(adv[0].width)\tunits_per_em\t\(CTFontGetUnitsPerEm(face))")

func table(_ tag: String) -> [UInt8]? {
    let t = tag.utf8.reduce(UInt32(0)) { $0 << 8 | UInt32($1) }
    guard let data = CTFontCopyTable(face, CTFontTableTag(t), CTFontTableOptions(rawValue: 0)) else { return nil }
    return [UInt8](data as Data)
}
func u16(_ b: [UInt8], _ o: Int) -> Int { o + 1 < b.count ? Int(b[o]) << 8 | Int(b[o + 1]) : 0 }
func u32(_ b: [UInt8], _ o: Int) -> Int { u16(b, o) << 16 | u16(b, o + 2) }
// OpenType ClassDef lookup (formats 1 and 2).
func classOf(_ b: [UInt8], _ at: Int, _ glyph: Int) -> Int {
    switch u16(b, at) {
    case 1:
        let start = u16(b, at + 2), count = u16(b, at + 4)
        return glyph >= start && glyph < start + count ? u16(b, at + 6 + 2 * (glyph - start)) : 0
    case 2:
        for r in 0..<u16(b, at + 2) {
            let o = at + 4 + 6 * r
            if glyph >= u16(b, o) && glyph <= u16(b, o + 2) { return u16(b, o + 4) }
        }
        return 0
    default: return 0
    }
}
var plan: [String] = []
if let gdef = table("GDEF") {
    let classDef = u16(gdef, 4)
    plan.append("gdef_glyph_classes=\(classDef != 0 ? 1 : 0)")
    plan.append("notdef_gdef_class=\(classDef != 0 ? classOf(gdef, classDef, 0) : 0)")
} else {
    plan.append("gdef_glyph_classes=0"); plan.append("notdef_gdef_class=0")
}
plan.append("gsub=\(table("GSUB") != nil ? 1 : 0)")
plan.append("gpos=\(table("GPOS") != nil ? 1 : 0)")
plan.append("morx=\(table("morx") != nil ? 1 : 0)")
plan.append("kerx=\(table("kerx") != nil ? 1 : 0)")
var kernFormats: [Int] = []
if let kern = table("kern") {
    if u16(kern, 0) == 0 {  // OpenType kern: u16 version, u16 nTables; coverage high byte = format
        var o = 4
        for _ in 0..<u16(kern, 2) { kernFormats.append(u16(kern, o + 4) >> 8); o += u16(kern, o + 2) }
    } else {                // Apple kern: fixed version, u32 nTables; coverage low byte = format
        var o = 8
        for _ in 0..<u32(kern, 4) { kernFormats.append(u16(kern, o + 4) & 0xFF); o += u32(kern, o) }
    }
    plan.append("kern=1")
} else {
    plan.append("kern=0")
}
plan.append("kern_formats=\(kernFormats.map(String.init).joined(separator: ","))")
plan.append("machine_kern=\(kernFormats.contains(1) ? 1 : 0)")
print("#hb_plan\t" + plan.joined(separator: "\t"))

var crossKern = 0
if let kern = table("kern") {
    if u16(kern, 0) == 0 {  // OpenType kern: u16 version, u16 nTables; coverage bit 2 CrossStream
        var o = 4
        for _ in 0..<u16(kern, 2) { if u16(kern, o + 4) & 0x04 != 0 { crossKern = 1 }; o += u16(kern, o + 2) }
    } else {                // Apple kern: fixed version, u32 nTables; coverage bit 0x4000 CrossStream
        var o = 8
        for _ in 0..<u32(kern, 4) { if u16(kern, o + 4) & 0x4000 != 0 { crossKern = 1 }; o += u32(kern, o) }
    }
}
// GSUB/GPOS header: u16 major, u16 minor, Offset16 ScriptList; ScriptList: u16 count, records (Tag, Offset16).
func scriptTags(_ tag: String) -> String {
    guard let t = table(tag) else { return "" }
    let list = u16(t, 4)
    return (0..<u16(t, list)).map { i -> String in
        let o = list + 2 + 6 * i
        return String(bytes: t[o..<o + 4], encoding: .ascii) ?? "?"
    }.joined(separator: ",")
}
print("#plan_extra\tcross_kern=\(crossKern)\ttrak=\(table("trak") != nil ? 1 : 0)\tSTAT=\(table("STAT") != nil ? 1 : 0)\tgsub_scripts=\(scriptTags("GSUB"))\tgpos_scripts=\(scriptTags("GPOS"))\tgdef_length=\(table("GDEF")?.count ?? 0)\tgsub_length=\(table("GSUB")?.count ?? 0)\tgpos_length=\(table("GPOS")?.count ?? 0)")

var run: (UInt32, UInt32)? = nil
func flush() {
    if let r = run { print("P\t\(String(r.0, radix: 16, uppercase: true))\t\(String(r.1, radix: 16, uppercase: true))") }
    run = nil
}
for cp in UInt32(0)...UInt32(0x10FFFF) {
    if cp >= 0xD800 && cp <= 0xDFFF { continue }
    var units = Array(String(Character(Unicode.Scalar(cp)!)).utf16)
    var glyphs = [CGGlyph](repeating: 0, count: units.count)
    let own = CTFontGetGlyphsForCharacters(face, &units, &glyphs, units.count) && glyphs[0] != 0
    if own, let r = run, r.1 + 1 == cp || (r.1 == 0xD7FF && cp == 0xE000) {
        run = (r.0, cp)
    } else {
        if run != nil && !own { flush() }
        if own { flush(); run = (cp, cp) }
    }
}
flush()
