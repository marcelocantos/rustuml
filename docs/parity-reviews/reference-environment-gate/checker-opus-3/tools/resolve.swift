import CoreText
import Foundation
for name in CommandLine.arguments.dropFirst() {
    let font = CTFontCreateWithName(name as CFString, 12, nil)
    let url = CTFontCopyAttribute(font, kCTFontURLAttribute) as? URL
    let ps = CTFontCopyPostScriptName(font) as String
    print("\(name)\t\(ps)\t\(url?.path ?? "-")")
}
