// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
//
// Environment extraction (not Java): resolve every Unicode scalar through the two mapping
// paths JDK 21's macOS font stack uses for a physical CFont, with system fonts only.
//
//  simple  (FontDesignMetrics, BMP text without complex characters): CCharToGlyphMapper ->
//          the font's own glyph, else CoreText substitution (CTS_CopyCTFallbackFontAndGlyphForUnicode;
//          here CTFontCreateForString). LastResort, no glyph, or a font outside the system
//          font directories counts as missing (glyph 0, the primary font's .notdef).
//  complex (TextLayout; any text holding a surrogate): CFont.createCompositeFont slots =
//          primary + CTFontCopyDefaultCascadeListForLanguages names + "GeezaPro", each resolved by
//          name as CFontManager.getOrCreateFallbackFont ('.'-prefixed names are dropped,
//          ".AppleSymbolsFB" -> "AppleSymbols"); a scalar is mapped when some slot has a glyph.
//
// Usage:
//   swiftc -O tools/FontChainExtract.swift -o /tmp/font-chain-extract
//   /tmp/font-chain-extract LucidaGrande > lucida.tsv   (one PostScript or family name per run)
// Then tools/font_chain_coverage.py turns the TSVs into crates/rustuml-render/src/font_chain_coverage.rs.
// Output: prints header lines then "S<TAB>start<TAB>end" (simple-path missing BMP runs)
// and "C<TAB>start<TAB>end" (complex-path mapped runs, all planes).
import CoreText
import Foundation
let family = CommandLine.arguments[1]
let size: CGFloat = 2048
let primary = CTFontCreateWithName(family as CFString, size, nil)
func psName(_ f: CTFont) -> String { CTFontCopyPostScriptName(f) as String }
func path(_ f: CTFont) -> String { (CTFontCopyAttribute(f, kCTFontURLAttribute) as? URL)?.path ?? "-" }
func isSystem(_ f: CTFont) -> Bool { let p = path(f); return p.hasPrefix("/System/Library/") || p.hasPrefix("/Library/Fonts/") }
var g0: [CGGlyph] = [0]; var adv = [CGSize](repeating: .zero, count: 1)
CTFontGetAdvancesForGlyphs(primary, .horizontal, &g0, &adv, 1)
print("#host\t\(ProcessInfo.processInfo.operatingSystemVersionString)")
print("#family\t\(family)\t\(psName(primary))\t\(path(primary))")
print("#notdef_advance_units\t\(adv[0].width)\tunits_per_em\t\(CTFontGetUnitsPerEm(primary))")
var names = (CTFontCopyDefaultCascadeListForLanguages(primary, CFLocaleCopyISOLanguageCodes()) as? [CTFontDescriptor] ?? []).map {
    CTFontDescriptorCopyAttribute($0, kCTFontNameAttribute) as? String ?? "" }
names.append("GeezaPro")
var slots: [CTFont] = [primary]
for var n in names {
    if n == ".AppleSymbolsFB" { n = "AppleSymbols" }
    if n.hasPrefix(".") { print("#slot_dropped\t\(n)"); continue }
    let f = CTFontCreateWithName(n as CFString, size, nil)
    if psName(f) != n { print("#slot_unresolved\t\(n)\t\(psName(f))"); continue }
    if slots.contains(where: { psName($0) == n }) { continue }
    print("#slot\t\(n)\t\(path(f))")
    slots.append(f)
}
func has(_ f: CTFont, _ units: inout [UniChar]) -> Bool {
    var g = [CGGlyph](repeating: 0, count: units.count)
    return CTFontGetGlyphsForCharacters(f, &units, &g, units.count) && g[0] != 0
}
var runs: [String: (UInt32, UInt32)] = [:]
func mark(_ k: String, _ cp: UInt32, _ on: Bool) {
    if let r = runs[k], r.1 + 1 == cp || (r.1 == 0xD7FF && cp == 0xE000) {
        if on { runs[k] = (r.0, cp); return }
        print("\(k)\t\(String(r.0, radix: 16, uppercase: true))\t\(String(r.1, radix: 16, uppercase: true))"); runs[k] = nil
    } else if let r = runs[k] {
        print("\(k)\t\(String(r.0, radix: 16, uppercase: true))\t\(String(r.1, radix: 16, uppercase: true))"); runs[k] = nil
    }
    if on && runs[k] == nil { runs[k] = (cp, cp) }
}
for cp in UInt32(0)...UInt32(0x10FFFF) {
    if cp >= 0xD800 && cp <= 0xDFFF { continue }
    let s = String(Character(Unicode.Scalar(cp)!))
    var units = Array(s.utf16)
    let inPrimary = has(primary, &units)
    var composite = inPrimary
    if !composite { for f in slots.dropFirst() where has(f, &units) { composite = true; break } }
    mark("C", cp, composite)
    if cp < 0x10000 {
        var simple = inPrimary
        if !simple {
            let fb = CTFontCreateForString(primary, s as CFString, CFRange(location: 0, length: units.count))
            simple = psName(fb) != "LastResort" && isSystem(fb) && has(fb, &units)
            if !simple && !isSystem(fb) { simple = composite } // a user font: system slots only
        }
        mark("S", cp, !simple)
    }
}
for (k, r) in runs { print("\(k)\t\(String(r.0, radix: 16, uppercase: true))\t\(String(r.1, radix: 16, uppercase: true))") }
