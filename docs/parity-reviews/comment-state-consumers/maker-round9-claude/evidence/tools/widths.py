"""Per case: label textLength (longest text containing the stem) and viewBox for java/base/merge/tip. Usage: widths.py <work> <label> [prefix]"""
import json, re, sys, pathlib
W, L = pathlib.Path(sys.argv[1]), sys.argv[2]; pre = sys.argv[3] if len(sys.argv) > 3 else ""
rows = json.load(open(W / f"batch-{L}.json")); out = {}
T = re.compile(r'<text[^>]*textLength="([^"]*)"[^>]*>([^<]*)</text>')
for n, r in rows.items():
    if not n.startswith(pre): continue
    d = {}
    for t in ["java", "base", "merge", "tip"]:
        s = (W / f"out-{L}" / f"{n}.{t}.svg").read_text(errors="replace")
        texts = T.findall(s); stem = [float(a) for a, b in texts if any(k in b for k in ("Quay", "Wharf", "Tide", "Harbor", "Reef", "Vault", "Pier", "Dock"))]
        vb = re.search(r'viewBox="0 0 ([^"]*)"', s)
        d[t] = (max(stem) if stem else None, vb.group(1).replace(" ", "x") if vb else None)
    d["strict"] = [r.get(f"{t}_strict", "")[:5] + str(r.get(f"{t}_ndiff", "")) for t in ["base", "merge", "tip"]]
    out[n] = d
    print(f"{n:34s}", " ".join(f"{t}:{d[t][0]}/{d[t][1]}" for t in ["java", "merge", "tip"]), d["strict"])
json.dump(out, open(W / f"widths-{L}.json", "w"), indent=1)
