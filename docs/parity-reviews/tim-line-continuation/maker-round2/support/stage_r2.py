"""Stage maker batch r2 (T14.22 round 2) as an immutable Java reference request.

Usage: python3 -I stage_r2.py PROBE_DIR BATCH_DIR
PROBE_DIR is gen_probes.py output. Writes BATCH_DIR/inputs/*, BATCH_DIR/reference-manifest.json and
BATCH_DIR/reference-input-bundle.json.gz. No Java is run here; validity is decided later from raw Java SVG + stderr.
"""
import base64, gzip, hashlib, json, sys
from pathlib import Path
probe, batch = Path(sys.argv[1]), Path(sys.argv[2])
batch.mkdir(exist_ok=False); inputs = batch / "inputs"; inputs.mkdir()
RESOURCES = ["r2_tail.iuml", "r2_tails.iuml", "r2_plain.iuml", "r2_link.iuml", "r2_comp.iuml"]
# A tab is allowed: r2_seq_tab_end_after_backslash tests a tab-indented end.
def clean(text, crlf):
    return not any(0xE000 <= ord(c) <= 0xE0FF or (ord(c) < 32 and c not in "\n\t" and not (crlf and c == "\r")) for c in text)
mc, files = [], []
for case in json.loads((probe / "cases.json").read_text()):
    name = case["name"]; data = (probe / f"{name}.puml").read_bytes(); text = data.decode()
    if case["intended_valid"]: assert clean(text, case["crlf"]), name
    if case["crlf"]: assert "\r\n" in text and "\n" not in text.replace("\r\n", ""), name
    (inputs / f"{name}.puml").write_bytes(data); h = hashlib.sha256(data).hexdigest()
    axes = case["axes"] + (["line-endings:CRLF"] if case["crlf"] else [])
    mc.append({"name": name, "source": str(inputs / f"{name}.puml"), "source_sha256": h, "cwd": "PRIVATE_CWD", "timezone": "GMT+11:00",
        "invocation": ["/Users/marcelo/rustuml-reference/bin/reference", "--cwd", "PRIVATE_CWD", "--timezone", "GMT+11:00", "--", "-pipe", "-tsvg"],
        "env": {}, "intended_valid": case["intended_valid"], "axes": axes, "mechanism": "tim-line-continuation"})
    files.append({"path": f"{name}.puml", "sha256": h, "base64": base64.b64encode(data).decode()})
assert len({c["name"] for c in mc}) == len(mc)
res = []
for path in RESOURCES:
    data = (probe / path).read_bytes(); assert clean(data.decode(), False), path
    (inputs / path).write_bytes(data)
    res.append({"path": path, "sha256": hashlib.sha256(data).hexdigest(), "base64": base64.b64encode(data).decode()})
manifest = {"schema_version": 1, "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
    "jar_sha256": "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053", "cases": mc}
mbytes = json.dumps(manifest, indent=1, ensure_ascii=False).encode()
(batch / "reference-manifest.json").write_bytes(mbytes)
bundle = {"manifest": manifest, "manifest_sha256": hashlib.sha256(mbytes).hexdigest(), "files": files, "resources": res}
with gzip.GzipFile(batch / "reference-input-bundle.json.gz", "wb", mtime=0) as fh:
    fh.write(json.dumps(bundle, ensure_ascii=False).encode())
print(len(mc), sum(c["intended_valid"] for c in mc), len(res))
