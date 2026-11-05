import CoreText
import Foundation
for n in CommandLine.arguments.dropFirst() {
    let f = CTFontCreateWithName(n as CFString, 2048, nil)
    var g: [CGGlyph] = [0]; var a = [CGSize](repeating: .zero, count: 1)
    CTFontGetAdvancesForGlyphs(f, .horizontal, &g, &a, 1)
    let p = (CTFontCopyAttribute(f, kCTFontURLAttribute) as? URL)?.path ?? "-"
    print("\(n)\t\(CTFontCopyPostScriptName(f) as String)\tupem=\(CTFontGetUnitsPerEm(f))\tnotdef=\(a[0].width)\t\(p)")
}
