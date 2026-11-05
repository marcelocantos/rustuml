"""Max/mean absolute numeric deviation per tree vs Java, element-aligned (same element count only).
Usage: numdev.py java.svg tree1.svg [tree2.svg ...]"""
import sys, re, xml.etree.ElementTree as ET
NUM = re.compile(r"-?\d+(?:\.\d+)?")
def flat(p):
    t = re.sub(r"<\?[^>]*\?>", "", open(p, encoding="utf-8", errors="replace").read())
    out = []
    for i, e in enumerate(ET.fromstring(t).iter()):
        if i == 0: continue  # root canvas excluded, as the strict comparator does
        for k, v in sorted(e.attrib.items()):
            if k in ("x", "y", "cx", "cy", "x1", "x2", "y1", "y2", "width", "height", "points", "d", "textLength"):
                out.extend((e.tag, k, float(n)) for n in NUM.findall(v))
    return out
j = flat(sys.argv[1])
for p in sys.argv[2:]:
    a = flat(p)
    if len(a) != len(j) or any(x[:2] != y[:2] for x, y in zip(j, a)): print(p.rsplit("/", 1)[-1], "structure differs"); continue
    d = [abs(x[2] - y[2]) for x, y in zip(j, a)]
    print(p.rsplit("/", 2)[-2][:20], "max %.4f  n>0.001 %d  n>0.01 %d / %d" % (max(d), sum(v > 0.001 for v in d), sum(v > 0.01 for v in d), len(d)))
