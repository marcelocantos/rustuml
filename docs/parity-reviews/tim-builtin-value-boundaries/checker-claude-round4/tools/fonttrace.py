"""Environment diagnostic (not a renderer constant): which macOS font Java's SansSerif resolves to, and the
advance of its missing glyph (.notdef, glyph 0). Usage: python3 -I fonttrace.py <out.json>
Identifies the font by matching RustUML's CHAR_WIDTHS_14 ASCII advances (Java-extracted) against hmtx."""
import json, sys, re, pathlib
from fontTools.ttLib import TTCollection, TTFont
REPO = pathlib.Path("/Users/marcelo/.t3/worktrees/rustuml/t3-d0bc7471")
src = (REPO / "crates/rustuml-render/src/plantuml_metrics.rs").read_text()
m = re.search(r"const CHAR_WIDTHS_14: \[f64; 95\] = \[(.*?)\];", src, re.S)
table = [float(x) for x in re.findall(r"-?\d+(?:\.\d+)?", m.group(1))]
out = {"rust_table": "CHAR_WIDTHS_14 (size 14 plain)", "fonts": []}
for path in ["/System/Library/Fonts/LucidaGrande.ttc", "/System/Library/Fonts/Helvetica.ttc",
             "/System/Library/Fonts/Supplemental/Arial.ttf", "/System/Library/Fonts/SFNS.ttf"]:
    fonts = TTCollection(path).fonts if path.endswith(".ttc") else [TTFont(path)]
    for f in fonts[:2]:
        upm = f["head"].unitsPerEm; hm = f["hmtx"]; cmap = f.getBestCmap()
        adv = [hm[cmap[c]][0] * 14 / upm if c in cmap else None for c in range(32, 127)]
        exact = sum(1 for a, b in zip(adv, table) if a is not None and abs(a - b) < 1e-9)
        g0 = f.getGlyphOrder()[0]
        out["fonts"].append({"file": path, "name": f["name"].getDebugName(4), "units_per_em": upm,
                             "ascii_advances_equal_to_rust_table": exact, "notdef_glyph": g0,
                             "notdef_advance_units": hm[g0][0], "notdef_advance_em": hm[g0][0] / upm,
                             "maps_U+10FFFD": 0x10FFFD in cmap, "maps_U+10FFFF": 0x10FFFF in cmap,
                             "maps_U+F0000": 0xF0000 in cmap, "maps_U+F000": 0xF000 in cmap})
json.dump(out, open(sys.argv[1], "w"), indent=1)
for f in out["fonts"]: print(f["name"], f["ascii_advances_equal_to_rust_table"], "/95", ".notdef", f["notdef_advance_units"], "/", f["units_per_em"], "=", f["notdef_advance_em"], "em")
