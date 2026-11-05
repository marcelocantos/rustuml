// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
//
// Environment extraction (not Java): resolve every Unicode scalar through the two mapping
// paths JDK 21's macOS font stack uses for a physical CFont, over every font the JDK sees
// (system, downloaded MobileAsset and user fonts alike).
//
//  simple  (FontDesignMetrics, BMP text without complex characters): CCharToGlyphMapper ->
//          CoreTextSupport.m CTS_GetGlyphsAsIntsForCharacters (JDK 21.0.5, 99-135): the font's
//          own glyph, else the glyph of JRSFontCreateFallbackFontForCharacters (called here
//          through dlsym, as the JDK links it), else 0 (missing: the primary .notdef).
//  complex (TextLayout; any text holding a surrogate): CFont.createCompositeFont (203-239)
//          slots = primary + CFont.getCascadeList (AWTFont.m 547-580:
//          CTFontCopyDefaultCascadeListForLanguages(font, CFLocaleCopyISOLanguageCodes()))
//          + "GeezaPro"; ".AppleSymbolsFB" -> "AppleSymbols"; each name through
//          CFontManager.getOrCreateFallbackFont (328-338): a font registered by
//          loadNativeFonts (AWTFont.m GetFilteredFonts: every face of every
//          NSFontManager family not starting with '.', plus the two JavaFX UI fonts), else
//          null for a '.' name, else a CFont created by name. CCompositeGlyphMapper
//          .convertToGlyph (54-67): the first slot whose own glyph is > 0.
//
// Each face also reports the sfnt facts that decide HarfBuzz 8.2.2's plan for its .notdef
// (hb-ot-shape.cc 136-189): GDEF glyph classes and the class of glyph 0, morx, kerx, and the
// kern subtable formats (format 1 is a state machine).
//
// Usage:
//   swiftc -O tools/FontChainExtract.swift -o /tmp/font-chain-extract
//   /tmp/font-chain-extract LucidaGrande > lucida.tsv   (one PostScript or family name per run)
//   /tmp/font-chain-extract --census > census.tsv       (every font loadNativeFonts registers)
// Then tools/font_chain_coverage.py turns the TSVs into crates/rustuml-render/src/font_chain_coverage.rs.
// Output: header lines, then "S<TAB>start<TAB>end" (simple-path missing BMP runs)
// and "C<TAB>start<TAB>end" (complex-path mapped runs, all planes).
import AppKit
import CoreText
import Foundation

let size: CGFloat = 2048
func psName(_ f: CTFont) -> String { CTFontCopyPostScriptName(f) as String }
func path(_ f: CTFont) -> String { (CTFontCopyAttribute(f, kCTFontURLAttribute) as? URL)?.path ?? "-" }
func location(_ p: String) -> String {
    if p.hasPrefix("/System/Library/AssetsV2/") { return "asset" }
    if p.hasPrefix("/System/Library/") { return "system" }
    if p.hasPrefix("/Library/Fonts/") { return "local" }
    if p.hasPrefix(NSHomeDirectory() + "/Library/Fonts/") { return "user" }
    return "other"
}

// AWTFont.m GetFilteredFonts (231-276): the names loadNativeFonts registers.
// The JavaFX UI fonts (addFont) cannot be created by name, so they are kept as fonts.
var uiFonts: [String: CTFont] = [:]
func registeredFonts() -> [(String, String)] {
    let manager = NSFontManager.shared
    var fonts: [(String, String)] = []
    for family in manager.availableFontFamilies where !family.hasPrefix(".") {
        for member in manager.availableMembers(ofFontFamily: family) ?? [] {
            if let face = member.first as? String { fonts.append((face, family)) }
        }
    }
    for kind in [CTFontUIFontType.system, CTFontUIFontType.emphasizedSystem] {
        if let ui = CTFontCreateUIFontForLanguage(kind, size, nil) {
            uiFonts[psName(ui)] = ui
            fonts.append((psName(ui), CTFontCopyFamilyName(ui) as String))
        }
    }
    return fonts
}
func font(named n: String) -> CTFont { uiFonts[n] ?? CTFontCreateWithName(n as CFString, size, nil) }

let registered = registeredFonts()
if CommandLine.arguments[1] == "--census" {
    print("#host\t\(ProcessInfo.processInfo.operatingSystemVersionString)")
    for (face, family) in registered.sorted(by: { $0.0 < $1.0 }) {
        let p = path(font(named: face))
        print("\(face)\t\(family)\t\(location(p))\t\(p)")
    }
    exit(0)
}
let registeredNames = Set(registered.map { $0.0 })

// JRSFontCreateFallbackFontForCharacters, the JDK's simple-path substitution.
typealias JRSFallback = @convention(c) (CTFont, UnsafePointer<UniChar>, CFIndex) -> Unmanaged<CTFont>?
guard let jrsHandle = dlopen("/System/Library/Frameworks/JavaRuntimeSupport.framework/JavaRuntimeSupport", RTLD_NOW),
      let jrsSymbol = dlsym(jrsHandle, "JRSFontCreateFallbackFontForCharacters") else {
    FileHandle.standardError.write("JRSFontCreateFallbackFontForCharacters unavailable\n".data(using: .utf8)!)
    exit(2)
}
let jrsFallback = unsafeBitCast(jrsSymbol, to: JRSFallback.self)

let family = CommandLine.arguments[1]
let primary = CTFontCreateWithName(family as CFString, size, nil)
var g0: [CGGlyph] = [0]; var adv = [CGSize](repeating: .zero, count: 1)
CTFontGetAdvancesForGlyphs(primary, .horizontal, &g0, &adv, 1)
print("#host\t\(ProcessInfo.processInfo.operatingSystemVersionString)")
print("#family\t\(family)\t\(psName(primary))\t\(path(primary))\t\(location(path(primary)))")
print("#notdef_advance_units\t\(adv[0].width)\tunits_per_em\t\(CTFontGetUnitsPerEm(primary))")

// HarfBuzz plan facts for the primary face (what hb-ot-shape.cc decides from the tables).
func table(_ f: CTFont, _ tag: String) -> [UInt8]? {
    let t = tag.utf8.reduce(UInt32(0)) { $0 << 8 | UInt32($1) }
    guard let data = CTFontCopyTable(f, CTFontTableTag(t), CTFontTableOptions(rawValue: 0)) else { return nil }
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
if let gdef = table(primary, "GDEF") {
    let classDef = u16(gdef, 4)
    plan.append("gdef_glyph_classes=\(classDef != 0 ? 1 : 0)")
    plan.append("notdef_gdef_class=\(classDef != 0 ? classOf(gdef, classDef, 0) : 0)")
} else {
    plan.append("gdef_glyph_classes=0"); plan.append("notdef_gdef_class=0")
}
plan.append("gsub=\(table(primary, "GSUB") != nil ? 1 : 0)")
plan.append("gpos=\(table(primary, "GPOS") != nil ? 1 : 0)")
plan.append("morx=\(table(primary, "morx") != nil ? 1 : 0)")
plan.append("kerx=\(table(primary, "kerx") != nil ? 1 : 0)")
var kernFormats: [Int] = []
if let kern = table(primary, "kern") {
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

let codes = CFLocaleCopyISOLanguageCodes()
print("#cascade_languages\t\(CFArrayGetCount(codes))")
var names = (CTFontCopyDefaultCascadeListForLanguages(primary, codes) as? [CTFontDescriptor] ?? []).map {
    CTFontDescriptorCopyAttribute($0, kCTFontNameAttribute) as? String ?? "" }
names.append("GeezaPro")
var slots: [CTFont] = [primary]
for var n in names {
    if n == ".AppleSymbolsFB" { n = "AppleSymbols" }
    if !registeredNames.contains(n) && n.hasPrefix(".") { print("#slot_dropped\t\(n)"); continue }
    let f = font(named: n)
    if psName(f) != n && !registeredNames.contains(n) { print("#slot_unresolved\t\(n)\t\(psName(f))"); continue }
    if psName(f) == psName(primary) || slots.contains(where: { psName($0) == psName(f) }) { continue }
    print("#slot\t\(n)\t\(path(f))\t\(location(path(f)))\t\(registeredNames.contains(n) ? "registered" : "by_name")")
    slots.append(f)
}
func has(_ f: CTFont, _ units: inout [UniChar]) -> Bool {
    var g = [CGGlyph](repeating: 0, count: units.count)
    return CTFontGetGlyphsForCharacters(f, &units, &g, units.count) && g[0] != 0
}
var simpleSources: [String: Int] = [:]
var runs: [String: (UInt32, UInt32)] = [:]
func flush(_ k: String) {
    if let r = runs[k] { print("\(k)\t\(String(r.0, radix: 16, uppercase: true))\t\(String(r.1, radix: 16, uppercase: true))") }
    runs[k] = nil
}
func mark(_ k: String, _ cp: UInt32, _ on: Bool) {
    if let r = runs[k], r.1 + 1 == cp || (r.1 == 0xD7FF && cp == 0xE000) {
        if on { runs[k] = (r.0, cp); return }
        flush(k)
    } else if runs[k] != nil {
        flush(k)
    }
    if on { runs[k] = (cp, cp) }
}
for cp in UInt32(0)...UInt32(0x10FFFF) {
    if cp >= 0xD800 && cp <= 0xDFFF { continue }
    var units = Array(String(Character(Unicode.Scalar(cp)!)).utf16)
    let inPrimary = has(primary, &units)
    var composite = inPrimary
    if !composite { for f in slots.dropFirst() where has(f, &units) { composite = true; break } }
    mark("C", cp, composite)
    if cp < 0x10000 {
        var simple = inPrimary
        if !simple, let fb = jrsFallback(primary, &units, units.count)?.takeRetainedValue() {
            simple = has(fb, &units)
            if simple { simpleSources[location(path(fb)), default: 0] += 1 }
        }
        mark("S", cp, !simple)
    }
}
flush("C"); flush("S")
for (k, n) in simpleSources.sorted(by: { $0.key < $1.key }) { print("#simple_substitutions\t\(k)\t\(n)") }
