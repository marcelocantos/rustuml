import CoreText
import Foundation
// For each requested family, report whether an installed font matches by family or PostScript name, and its file.
let coll = CTFontCollectionCreateFromAvailableFonts(nil)
let descs = (CTFontCollectionCreateMatchingFontDescriptors(coll) as? [CTFontDescriptor]) ?? []
var byFamily: [String: Set<String>] = [:]
for d in descs {
    let fam = (CTFontDescriptorCopyAttribute(d, kCTFontFamilyNameAttribute) as? String ?? "").lowercased()
    let ps = (CTFontDescriptorCopyAttribute(d, kCTFontNameAttribute) as? String ?? "").lowercased()
    let url = (CTFontDescriptorCopyAttribute(d, kCTFontURLAttribute) as? URL)?.path ?? "-"
    byFamily[fam, default: []].insert(url)
    byFamily[ps, default: []].insert(url)
}
while let line = readLine() {
    let key = line.trimmingCharacters(in: .whitespaces).lowercased()
    if key.isEmpty { continue }
    if let files = byFamily[key] { for f in files.sorted() { print("\(key)\tinstalled\t\(f)") } }
    else { print("\(key)\tnot-installed\t-") }
}
