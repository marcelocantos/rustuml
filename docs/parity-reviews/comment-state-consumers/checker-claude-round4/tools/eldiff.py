"""Element-by-element diff of two SVGs (document order). Usage: eldiff.py expected actual [max]"""
import sys, re, xml.etree.ElementTree as ET
def els(p):
    t = open(p, encoding="utf-8", errors="replace").read()
    t = re.sub(r"<\?[^>]*\?>", "", t)
    root = ET.fromstring(t)
    out = []
    for e in root.iter():
        tag = e.tag.split('}')[-1]
        out.append((tag, dict(e.attrib), (e.text or "").strip()))
    return out
a, b = els(sys.argv[1]), els(sys.argv[2]); mx = int(sys.argv[3]) if len(sys.argv) > 3 else 40
print("count", len(a), len(b)); n = 0
for i in range(max(len(a), len(b))):
    x = a[i] if i < len(a) else None; y = b[i] if i < len(b) else None
    if x != y:
        n += 1
        if n > mx: break
        if x and y and x[0] == y[0]:
            d = {k: (x[1].get(k), y[1].get(k)) for k in set(x[1]) | set(y[1]) if x[1].get(k) != y[1].get(k)}
            print(i, x[0], d, ("text", x[2], y[2]) if x[2] != y[2] else "")
        else:
            print(i, "J:", x and (x[0], x[1].get("id") or x[1].get("data-qualified-name"), x[2][:30]), "| R:", y and (y[0], y[1].get("id") or y[1].get("data-qualified-name"), y[2][:30]))
