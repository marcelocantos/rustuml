"""Rust-only structural invariants on state SVG output: negative coords; each data-qualified-name child
inside its parent's drawn rect (cluster or entity image). Usage: geomcheck.py <svg>..."""
import re, sys
for f in sys.argv[1:]:
    t = open(f, encoding="utf-8", errors="replace").read()
    probs = []
    for m in re.finditer(r'\b(x|y|cx|cy|x1|x2|y1|y2)="(-[0-9.]+)"', t): probs.append(f"negative {m.group(1)}={m.group(2)}")
    for m in re.finditer(r'd="M(-[0-9.]+),', t): probs.append(f"negative path start {m.group(1)}")
    rects = {}
    for g in re.finditer(r'<g class="(cluster|entity)" data-qualified-name="([^"]+)"[^>]*>(.*?)(?=<g class=|$)', t, re.S):
        r = re.search(r'<rect[^>]*height="([0-9.]+)"[^>]*width="([0-9.]+)" x="([0-9.]+)" y="([0-9.]+)"', g.group(3))
        if r: rects[g.group(2)] = (g.group(1), float(r.group(3)), float(r.group(4)), float(r.group(2)), float(r.group(1)))
    for name, (kind, x, y, w, h) in rects.items():
        if "." in name:
            parent = name.rsplit(".", 1)[0]
            if parent in rects:
                pk, px, py, pw, ph = rects[parent]
                if not (px - 1 <= x and x + w <= px + pw + 1 and py - 1 <= y and y + h <= py + ph + 1):
                    probs.append(f"{name} ({kind} {x},{y},{w}x{h}) outside parent {parent} ({pk} {px},{py},{pw}x{ph})")
    comments = re.findall(r"<!--(.*?)-->", t)
    print(f.split('/')[-1], "OK" if not probs else "VIOLATION", "|", "; ".join(probs[:4]), "| comments:", comments[:6])
