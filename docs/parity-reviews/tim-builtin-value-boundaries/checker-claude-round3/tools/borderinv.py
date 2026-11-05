"""Java-derived invariant (Cluster.manageEntryExitPoint/FrontierCalculator + EntityImageStateBorder): every
entry/exit point circle (ellipse rx=ry=6, stroke-width 1.5) in a STATE SVG is centred on the border of some
drawn cluster rectangle. Usage: borderinv.py <svg>...  -> 'path OK n' or 'path VIOLATION [(cx,cy)...]'."""
import sys, re, xml.etree.ElementTree as ET
NS = "{http://www.w3.org/2000/svg}"
def check(path):
    t = open(path, encoding="utf-8", errors="replace").read()
    if 'data-diagram-type="STATE"' not in t or "<ellipse" not in t: return None
    try: root = ET.fromstring(re.sub(r"<\?[^>]*\?>", "", t))
    except ET.ParseError: return None
    rects = []
    for g in root.iter(NS + "g"):
        if g.get("class") == "cluster":
            for r in g.findall(NS + "rect"):
                if r.get("width"): rects.append(tuple(float(r.get(k)) for k in ("x", "y", "width", "height")))
    pts = [(float(e.get("cx")), float(e.get("cy"))) for e in root.iter(NS + "ellipse")
           if e.get("rx") == "6" and e.get("ry") == "6" and "stroke-width:1.5" in (e.get("style") or "")]
    if not pts or not rects: return None
    def on(cx, cy):
        for x, y, w, h in rects:
            if (abs(cy - y) < 0.02 or abs(cy - y - h) < 0.02) and x - 0.02 <= cx <= x + w + 0.02: return True
            if (abs(cx - x) < 0.02 or abs(cx - x - w) < 0.02) and y - 0.02 <= cy <= y + h + 0.02: return True
        return False
    bad = [p for p in pts if not on(*p)]
    return len(pts), bad
ok = bad_files = 0
for p in sys.argv[1:]:
    r = check(p)
    if r is None: continue
    n, bad = r
    if bad: bad_files += 1; print(p, "VIOLATION", bad)
    else: ok += 1
print("files OK", ok, "violations", bad_files, file=sys.stderr)
