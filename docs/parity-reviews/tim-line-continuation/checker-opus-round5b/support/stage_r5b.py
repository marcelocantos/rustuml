"""Stage checker batch R5B as an immutable Java reference request (no Java is run here).
Usage: python3 -I stage_r5b.py PROBE_DIR BATCH_DIR"""
import base64, gzip, hashlib, json, sys
from pathlib import Path
probe, batch = Path(sys.argv[1]), Path(sys.argv[2])
batch.mkdir(exist_ok=False); inputs = batch / "inputs"; inputs.mkdir()
INV = ["/Users/marcelo/rustuml-reference/bin/reference", "--cwd", "PRIVATE_CWD", "--timezone", "GMT+11:00", "--", "-pipe", "-tsvg"]
mc, files = [], []
for case in json.loads((probe / "cases.json").read_text()):
    name = case["name"]; data = (probe / f"{name}.puml").read_bytes()
    (inputs / f"{name}.puml").write_bytes(data); h = hashlib.sha256(data).hexdigest()
    mc.append({"name": name, "source": str(inputs / f"{name}.puml"), "source_sha256": h, "cwd": "PRIVATE_CWD", "timezone": "GMT+11:00",
        "invocation": INV, "env": {}, "intended_valid": case["intended_valid"],
        "axes": case["axes"] + (["line-endings:CRLF"] if case["crlf"] else []), "mechanism": "tim-line-continuation-round5-recheck"})
    files.append({"path": f"{name}.puml", "sha256": h, "base64": base64.b64encode(data).decode()})
res = []
for path in json.loads((probe / "resources.json").read_text()):
    data = (probe / path).read_bytes(); (inputs / path).write_bytes(data)
    res.append({"path": path, "sha256": hashlib.sha256(data).hexdigest(), "base64": base64.b64encode(data).decode()})
manifest = {"schema_version": 1, "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
    "jar_sha256": "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053", "cases": mc}
mbytes = json.dumps(manifest, indent=1, ensure_ascii=False).encode()
(batch / "reference-manifest.json").write_bytes(mbytes)
bundle = {"manifest": manifest, "manifest_sha256": hashlib.sha256(mbytes).hexdigest(), "files": files, "resources": res}
with gzip.GzipFile(batch / "reference-input-bundle.json.gz", "wb", mtime=0) as fh:
    fh.write(json.dumps(bundle, ensure_ascii=False).encode())
print(len(mc), sum(c["intended_valid"] for c in mc), len(res))
