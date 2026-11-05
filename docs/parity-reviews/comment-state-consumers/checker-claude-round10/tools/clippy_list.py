"""List error-level clippy/rustc diagnostics from cargo --message-format=json logs, per tree and crate.
Usage: python3 -I clippy_list.py <gates dir> <out.json>"""
import json, pathlib, sys, collections
G = pathlib.Path(sys.argv[1]); res = {}
for r in ["95291dc6b", "e24240b35"]:
    for k in ["rustuml-render", "rustuml-parser", "rustuml-layout", "rustuml-oracle", "rustuml", "rustuml-math"]:
        diags = []
        for line in open(G / f"{r}.clippy-{k}.json"):
            try: m = json.loads(line)
            except Exception: continue
            if m.get("reason") != "compiler-message": continue
            if not m["target"]["name"].replace("_", "-") == k and m["package_id"].split("#")[0].split("/")[-1] != k: pass
            msg = m["message"]
            if msg["level"] != "error" or not msg.get("spans"): continue
            sp = [s for s in msg["spans"] if s["is_primary"]][0]
            diags.append({"pkg": m["package_id"].split("#")[0].split("/")[-1], "code": (msg.get("code") or {}).get("code"),
                          "file": sp["file_name"], "line": sp["line_start"], "text": (sp.get("text") or [{}])[0].get("text", "").strip()[:160],
                          "message": msg["message"][:160]})
        res[f"{r}/{k}"] = diags
json.dump(res, open(sys.argv[2], "w"), indent=1)
for key, d in res.items():
    print(key, len(d), dict(collections.Counter(x["pkg"] for x in d)))
