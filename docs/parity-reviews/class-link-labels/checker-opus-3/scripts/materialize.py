"""Write a relay bundle's sources (and resources) into <dir>/src and a local manifest with absolute paths.
usage: materialize.py <relay-dir> <outdir>"""
import base64, gzip, hashlib, json, os, sys
relay, out = sys.argv[1], sys.argv[2]
b = json.load(gzip.open(os.path.join(relay, "reference-input-bundle.json.gz")))
m = b["manifest"] if isinstance(b["manifest"], dict) else json.load(open(os.path.join(relay, "reference-manifest.json")))
src = os.path.join(out, "src"); os.makedirs(src, exist_ok=True)
for f in b["files"] + b.get("resources", []):
    data = base64.b64decode(f["base64"]); assert hashlib.sha256(data).hexdigest() == f["sha256"]
    p = os.path.join(src, f["path"]); os.makedirs(os.path.dirname(p), exist_ok=True); open(p, "wb").write(data)
for c in m["cases"]:
    c["source"] = os.path.join(src, os.path.basename(c["source"]))
json.dump(m, open(os.path.join(out, "manifest.json"), "w"), indent=1)
print(len(m["cases"]), "cases;", len(b.get("resources", [])), "resources")
