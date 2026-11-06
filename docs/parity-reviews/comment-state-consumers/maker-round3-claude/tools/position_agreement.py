"""Exact x/y agreement of rect/text/line/ellipse elements with the Java golden, over outputs changed between two sweeps.
Usage: python3 -I position_agreement.py <repo> <events.json> <out-a> <out-b>"""
import json, re, sys
repo, ev, oa, ob = sys.argv[1:]
A = re.compile(r'<(rect|text|line|ellipse)\b([^>]*)>')
def attrs(s):
    out = []
    for m in A.finditer(s):
        a = dict(re.findall(r'(\w[\w:-]*)="([^"]*)"', m.group(2)))
        out.append((m.group(1), a.get('x', a.get('cx', a.get('x1'))), a.get('y', a.get('cy', a.get('y1')))))
    return out
tot = ta = tb = closer = farther = same = 0; far = []
for r in json.load(open(ev))['events']['bytes_changed']:
    j = open(f"{repo}/{r[:-5]}.svg", errors='replace').read()
    if 'An error has occured' in j or 'Syntax Error' in j: continue
    J = attrs(j); P = attrs(open(f"{oa}/{r[:-5]}.svg", errors='replace').read()); T = attrs(open(f"{ob}/{r[:-5]}.svg", errors='replace').read())
    ea = sum(x == y for x, y in zip(J, P)); eb = sum(x == y for x, y in zip(J, T))
    tot += len(J); ta += ea; tb += eb
    if eb > ea: closer += 1
    elif eb < ea: farther += 1; far.append((r, ea, eb))
    else: same += 1
print(f"agreement {ta} -> {tb} of {tot}; outputs closer {closer}, farther {farther}, same {same}")
for f in far: print("  FARTHER", *f)
