import CoreText
import Foundation
let fams = (CTFontManagerCopyAvailableFontFamilyNames() as? [String]) ?? []
let ps = (CTFontManagerCopyAvailablePostScriptNames() as? [String]) ?? []
for n in ["Courier","Times","Times New Roman","Courier New","Times-Roman","Courier-Bold"] {
  print("\(n)\tfamily=\(fams.contains(n))\tps=\(ps.contains(n))")
}
