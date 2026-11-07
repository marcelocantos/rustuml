import CoreText
import Foundation
// Mirrors sun.font.CFont.getCascadeList: CTFontCopyDefaultCascadeListForLanguages(font, CFLocaleCopyISOLanguageCodes()).
let codes = CFLocaleCopyISOLanguageCodes()
for ps in CommandLine.arguments.dropFirst() {
    let font = CTFontCreateWithName(ps as CFString, 12, nil)
    let own = (CTFontCopyAttribute(font, kCTFontURLAttribute) as? URL)?.path ?? "-"
    print("#face\t\(ps)\t\(CTFontCopyPostScriptName(font) as String)\t\(own)\tlanguages=\(CFArrayGetCount(codes))")
    let list = (CTFontCopyDefaultCascadeListForLanguages(font, codes) as? [CTFontDescriptor]) ?? []
    for d in list {
        let name = CTFontDescriptorCopyAttribute(d, kCTFontNameAttribute) as? String ?? "?"
        var url = (CTFontDescriptorCopyAttribute(d, kCTFontURLAttribute) as? URL)?.path
        if url == nil {
            let f = CTFontCreateWithFontDescriptor(d, 12, nil)
            url = (CTFontCopyAttribute(f, kCTFontURLAttribute) as? URL)?.path
        }
        print("#slot\t\(name)\t\(url ?? "-")")
    }
}
