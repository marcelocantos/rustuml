"""Stage an immutable Java reference batch (manifest + input bundle). Usage: python3 -I stage.py <batch_dir>"""
import base64, gzip, hashlib, json, pathlib, sys
D = pathlib.Path(sys.argv[1]); cases = json.load(open(D / "cases.json"))["cases"]
sha = lambda b: hashlib.sha256(b).hexdigest(); mc, files = [], []
for c in cases:
    src = D / "sources" / f"{c['name']}.puml"; data = src.read_bytes()
    mc.append({"name": c["name"], "source": str(src), "source_sha256": sha(data), "cwd": "PRIVATE_CWD", "timezone": "GMT+11:00",
               "invocation": ["/Users/marcelo/rustuml-reference/bin/reference", "--cwd", "PRIVATE_CWD", "--timezone", "GMT+11:00", "--", "-pipe", "-tsvg"],
               "env": {}, "intended_valid": c["intended_valid"], "axes": c["axes"]})
    files.append({"path": f"{c['name']}.puml", "sha256": sha(data), "base64": base64.b64encode(data).decode()})
manifest = {"schema_version": 1, "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
            "jar_sha256": "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053", "cases": mc}
mtext = json.dumps(manifest, indent=1, ensure_ascii=False).encode(); (D / "reference-manifest.json").write_bytes(mtext)
with open(D / "reference-input-bundle.json.gz", "wb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", mtime=0) as f:
    f.write(json.dumps({"manifest": manifest, "manifest_sha256": sha(mtext), "files": files, "resources": []}, ensure_ascii=False).encode())
for n in ["reference-manifest.json", "reference-input-bundle.json.gz", "rust-predictions.json"]: print(n, sha((D / n).read_bytes()))
