"""For every case whose output changed merge->tip: each <text> (aligned by document order) whose textLength
changed, with Java's textLength for the same-index <text> of the same content (else the first Java <text> of
the same content; None when Java draws no such text).
Usage: python3 -I tl_eval.py <work> <label> <tiptag> <out.json>"""
import json, re, sys, pathlib
W = pathlib.Path(sys.argv[1]); L = sys.argv[2]; TAG = sys.argv[3]
d = json.load(open(W / f"batch-{L}.json")); out = {}
T = re.compile(r'<text ([^>]*)>([^<]*)</text>')
def texts(p):
    return [(dict(re.findall(r'([\w-]+)="([^"]*)"', a)), b) for a, b in T.findall(p.read_text(errors="replace"))]
for n, r in d.items():
    if r["merge_sha256"] == r[f"{TAG}_sha256"]:
        continue
    j = texts(W / f"out-{L}/{n}.java.svg"); m = texts(W / f"out-{L}/{n}.merge.svg"); t = texts(W / f"out-{L}/{n}.{TAG}.svg")
    rows = []
    for i, ((ma, mb), (ta, tb)) in enumerate(zip(m, t)):
        if ma.get("textLength") != ta.get("textLength") or mb != tb:
            ja = j[i][0] if i < len(j) and j[i][1] == tb else next((a for a, b in j if b == tb), None)
            rows.append({"index": i, "text": tb, "family": ta.get("font-family"), "style": ta.get("font-style"),
                         "weight": ta.get("font-weight"), "merge": ma.get("textLength"), "tip": ta.get("textLength"),
                         "java": ja.get("textLength") if ja else None,
                         "tip_eq_java": bool(ja) and ja.get("textLength") == ta.get("textLength")})
    out[n] = {"strict_merge": r["merge_strict"], "strict_tip": r[f"{TAG}_strict"], "changed_texts": rows,
              "every_changed_text_eq_java": all(x["tip_eq_java"] for x in rows), "len_merge_tip": [len(m), len(t)]}
json.dump(out, open(sys.argv[4], "w"), indent=1, ensure_ascii=False)
for n, v in out.items():
    bad = [x for x in v["changed_texts"] if not x["tip_eq_java"]]
    print(f"{n:28s} {v['strict_merge']}->{v['strict_tip']} changed={len(v['changed_texts'])} eq_java={v['every_changed_text_eq_java']}", [(x['text'][:30], x['merge'], x['tip'], x['java']) for x in bad[:2]])
