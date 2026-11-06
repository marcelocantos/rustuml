import CoreText
import Foundation
typealias JRSFallback = @convention(c) (CTFont, UnsafePointer<UniChar>, CFIndex) -> Unmanaged<CTFont>?
let h = dlopen("/System/Library/Frameworks/JavaRuntimeSupport.framework/JavaRuntimeSupport", RTLD_NOW)
guard let sym = dlsym(h, "JRSFontCreateFallbackFontForCharacters") else { print("no symbol", String(cString: dlerror())); exit(1) }
let jrs = unsafeBitCast(sym, to: JRSFallback.self)
let primary = CTFontCreateWithName(CommandLine.arguments[1] as CFString, 2048, nil)
for a in CommandLine.arguments.dropFirst(2) {
    let cp = UInt32(a, radix: 16)!
    var u = Array(String(Character(Unicode.Scalar(cp)!)).utf16)
    var g = [CGGlyph](repeating: 0, count: u.count)
    let fb = jrs(primary, &u, u.count)?.takeRetainedValue()
    var name = "nil", path = "-", has = false
    if let fb = fb {
        name = CTFontCopyPostScriptName(fb) as String
        path = (CTFontCopyAttribute(fb, kCTFontURLAttribute) as? URL)?.path ?? "-"
        has = CTFontGetGlyphsForCharacters(fb, &u, &g, u.count) && g[0] != 0
    }
    let ctfs = CTFontCreateForString(primary, String(Character(Unicode.Scalar(cp)!)) as CFString, CFRange(location: 0, length: u.count))
    print("U+\(a)\tjrs=\(name)\tglyph=\(has)\t\(path)\tctfs=\(CTFontCopyPostScriptName(ctfs) as String)")
}
