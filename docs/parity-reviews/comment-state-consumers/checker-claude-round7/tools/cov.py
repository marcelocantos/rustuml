"""Parse the tip's generated font tables. Usage: python3 -I cov.py <repo> <code-hex>..."""
import re, sys, pathlib, json
R = pathlib.Path(sys.argv[1])
cov = (R / "crates/rustuml-render/src/font_chain_coverage.rs").read_text()
uni = (R / "crates/rustuml-render/src/font_chain_unicode.rs").read_text()
def runs_of(text, const_name):
    i = text.index(const_name); j = text.index("];", i)
    return [(int(a, 16), int(b, 16)) for a, b in re.findall(r"\(0x([0-9A-Fa-f]+),\s*0x([0-9A-Fa-f]+)\)", text[i:j])]
faces = {}
for m in re.finditer(r"pub\(crate\) const ([A-Z_]+): FaceCoverage = FaceCoverage \{(.*?)\n\};", cov, re.S):
    body = m.group(2); f = {}
    for key in ["simple_missing", "complex_mapped", "primary_mapped"]:
        v = re.search(key + r":\s*([A-Za-z0-9_:]+|&\[.*?\]),", body, re.S).group(1)
        f[key] = runs_of(cov, "const " + v.split("::")[-1] + ":") if not v.startswith("&[") else [(int(a,16),int(b,16)) for a,b in re.findall(r"\(0x([0-9A-Fa-f]+),\s*0x([0-9A-Fa-f]+)\)", v)]
    faces[m.group(1)] = f
inr = lambda runs, c: any(a <= c <= b for a, b in runs)
mn = runs_of(uni, "NONSPACING_MARKS"); marks = runs_of(uni, "UNICODE_MARKS")
for h in sys.argv[2:]:
    c = int(h, 16)
    print(f"U+{c:04X} Mn={inr(mn,c)} mark={inr(marks,c)} " + " ".join(f"{n[:6]}:{'P' if inr(f['primary_mapped'],c) else ('o' if inr(f['complex_mapped'],c) else '.')}" for n, f in faces.items()))
