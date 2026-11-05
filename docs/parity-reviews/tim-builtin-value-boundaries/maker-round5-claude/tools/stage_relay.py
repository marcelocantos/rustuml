"""Stage an immutable relay batch: reference-manifest.json, reference-input-bundle.json.gz and the Rust
predictions of the given build (registered before any Java result exists).
Usage: python3 -I stage_relay.py <batch_dir> <rust_bin> <rust_revision>"""
import base64, gzip, hashlib, json, os, pathlib, subprocess, sys, tempfile
D, BIN, REV = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3]
cases = json.load(open(D / "cases.json"))
JAVA_REV = "71806a23780b04a5ccde2f8ceb5121edad5eb711"
JAR = "aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053"
sha = lambda b: hashlib.sha256(b).hexdigest()
man_cases, files, preds = [], [], []
for c in cases:
    src = D / "sources" / f"{c['name']}.puml"; data = src.read_bytes()
    man_cases.append({"name": c["name"], "source": str(src), "source_sha256": sha(data), "cwd": "PRIVATE_CWD",
                      "timezone": "GMT+11:00",
                      "invocation": ["/Users/marcelo/rustuml-reference/bin/reference", "--cwd", "PRIVATE_CWD",
                                     "--timezone", "GMT+11:00", "--", "-pipe", "-tsvg"],
                      "env": {}, "intended_valid": c["intended_valid"], "axes": c["axes"]})
    files.append({"path": f"{c['name']}.puml", "sha256": sha(data), "base64": base64.b64encode(data).decode()})
    with tempfile.TemporaryDirectory() as cwd:
        p = subprocess.run([BIN, "-pipe", "-tsvg"], input=data, capture_output=True, cwd=cwd, timeout=120,
                           env={"PATH": "/usr/bin:/bin:/opt/homebrew/bin", "TZ": "GMT+11:00",
                                "RUSTUML_DEBUG": "date=1774210426000,tz=AEDT+1100"})
    preds.append({"name": c["name"], "exit": p.returncode, "stdout_sha256": sha(p.stdout),
                  "stdout_base64": base64.b64encode(p.stdout).decode(),
                  "stderr": p.stderr.decode("utf-8", "replace")[:400], "panic": b"panicked at" in p.stderr})
manifest = {"schema_version": 1, "java_revision": JAVA_REV, "jar_sha256": JAR, "cases": man_cases}
mtext = json.dumps(manifest, indent=1, ensure_ascii=False).encode()
(D / "reference-manifest.json").write_bytes(mtext)
bundle = {"manifest": manifest, "manifest_sha256": sha(mtext), "files": files, "resources": []}
with open(D / "reference-input-bundle.json.gz", "wb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", mtime=0) as f:
    f.write(json.dumps(bundle, ensure_ascii=False).encode())
(D / "rust-predictions.json").write_text(json.dumps({"schema_version": 1, "rust_revision": REV, "registered_before_java": True,
                                                    "predictions": preds}, indent=1, ensure_ascii=False))
for name in ["reference-manifest.json", "reference-input-bundle.json.gz", "rust-predictions.json"]:
    print(name, sha((D / name).read_bytes()))
print(len(cases), "cases; exits", {e: sum(1 for p in preds if p["exit"] == e) for e in {p["exit"] for p in preds}})
