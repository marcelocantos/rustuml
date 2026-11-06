"""Pre-register Rust outputs for a staged batch. Usage: python3 -I predict11.py <batch_dir> <tag>=<bin> ...
Each case runs from a private empty cwd (as the reference's PRIVATE_CWD); writes <batch_dir>/rust-predictions.json."""
import base64, hashlib, json, pathlib, subprocess, sys, tempfile
D = pathlib.Path(sys.argv[1]); bins = [a.split("=", 1) for a in sys.argv[2:]]
cases = json.load(open(D / "cases.json"))["cases"]; out = {"bins": {}, "cases": {}}
for t, b in bins: out["bins"][t] = {"path": b, "sha256": hashlib.sha256(open(b, "rb").read()).hexdigest()}
cwd = tempfile.mkdtemp(prefix="m11a-cwd-")
for c in cases:
    src = (D / "sources" / f"{c['name']}.puml").read_bytes(); row = {}
    for t, b in bins:
        p = subprocess.run([b, "-pipe", "-tsvg"], input=src, capture_output=True, cwd=cwd, timeout=60,
                           env={"PATH": "/usr/bin:/bin", "RUSTUML_DEBUG": "date=1774210426000,tz=AEDT+1100", "TZ": "GMT+11:00"})
        row[t] = {"exit": p.returncode, "stderr": p.stderr.decode("utf-8", "replace")[:300],
                  "stdout_sha256": hashlib.sha256(p.stdout).hexdigest(), "stdout_base64": base64.b64encode(p.stdout).decode()}
    out["cases"][c["name"]] = row
(D / "rust-predictions.json").write_text(json.dumps(out, indent=1))
tags = [t for t, _ in bins]
ch = [n for n, r in out["cases"].items() if len({r[t]["stdout_sha256"] for t in tags}) > 1]
print(len(cases), "cases;", len(ch), "outputs differ across", tags, "; nonzero exits:",
      [(n, t) for n, r in out["cases"].items() for t in tags if r[t]["exit"] != 0])
print("changed:", ch)
