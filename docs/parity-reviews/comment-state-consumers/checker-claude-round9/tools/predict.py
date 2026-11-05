"""Record Rust outputs of each probe before any Java run. Usage: python3 -I predict.py <batchdir> <tag>=<bin> ...
Each case runs from a private empty cwd (PRIVATE_CWD), -pipe -tsvg, harness date/tz pins."""
import base64, hashlib, json, pathlib, re, subprocess, sys
B = pathlib.Path(sys.argv[1]); BINS = [a.split("=", 1) for a in sys.argv[2:]]
cases = json.load(open(B / "cases.json"))["cases"]; cwd = B / "private-cwd"; cwd.mkdir(exist_ok=True)
STDLIB = str(pathlib.Path.home() / "work/github.com/plantuml/plantuml-stdlib/stdlib")
C = re.compile(r"<!--(.*?)-->", re.S); out = {"schema_version": 1, "registered_before_java": True, "builds": {}, "predictions": []}
for tag, b in BINS: out["builds"][tag] = {"path": b, "sha256": hashlib.sha256(open(b, "rb").read()).hexdigest()}
for c in cases:
    src = (B / "sources" / f"{c['name']}.puml").read_bytes(); row = {"name": c["name"], "source_sha256": hashlib.sha256(src).hexdigest()}
    for tag, b in BINS:
        p = subprocess.run([b, "-pipe", "-tsvg"], input=src, capture_output=True, cwd=cwd, timeout=60,
                           env={"PATH": "/usr/bin:/bin:/opt/homebrew/bin", "RUSTUML_DEBUG": "date=1774210426000,tz=AEDT+1100", "TZ": "GMT+11:00", "PLANTUML_STDLIB": STDLIB})
        t = p.stdout.decode("utf-8", "replace"); vb = re.search(r'viewBox="([^"]*)"', t)
        row[tag] = {"exit": p.returncode, "stdout_sha256": hashlib.sha256(p.stdout).hexdigest(), "stdout_base64": base64.b64encode(p.stdout).decode(),
                    "stderr": p.stderr.decode("utf-8", "replace")[:300], "panic": b"panicked at" in p.stderr,
                    "viewBox": vb.group(1) if vb else None, "comments": C.findall(t), "textLengths": re.findall(r'textLength="([^"]*)"', t)[:6]}
    out["predictions"].append(row)
(B / "rust-predictions.json").write_text(json.dumps(out, indent=1, ensure_ascii=False))
for r in out["predictions"]:
    tags = [t for t, _ in BINS]
    print(f"{r['name']:34s}", " ".join(f"{t}:{r[t]['exit']}{'P' if r[t]['panic'] else ''}:{(r[t]['viewBox'] or '-').replace('0 0 ','')}:{','.join(r[t]['textLengths'][:2])}" for t in tags),
          "| ev!=tip" if r.get("tip") and r.get("tip-ev") and r["tip"]["stdout_sha256"] != r["tip-ev"]["stdout_sha256"] else "")
