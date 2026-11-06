"""Run fresh Java relay batches through each RustUML build and compare whole output.

Usage: python3 -I batch.py <work> <batch_dir> <label> <bin>=<tag> [...]
Each case runs from a private cwd that holds the batch resources (PRIVATE_CWD).
Output: <work>/batch-<label>.json
"""
import base64, hashlib, json, pathlib, re, subprocess, sys

W = pathlib.Path(sys.argv[1]); B = pathlib.Path(sys.argv[2]); LABEL = sys.argv[3]
BINS = [a.split("=", 1) for a in sys.argv[4:]]
CMP = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round4-claude-maker/cmp/target/release/checker-cmp"
C = re.compile(r"<!--(.*?)-->", re.S)
DEBUG = "date=1774210426000,tz=AEDT+1100"
ERRMARK = ["Syntax Error", "Welcome to PlantUML", "An error has occured", "[From string", "(Assumed diagram type:"]

res = json.load(open(B / "reference-results.json"))
man = {c["name"]: c for c in json.load(open(B / "reference-manifest.json"))["cases"]}
cwd = W / f"cwd-{LABEL}"; cwd.mkdir(parents=True, exist_ok=True)
for r in res.get("resources", []):
    data = next(base64.b64decode(x["base64"]) for x in __import__("json").load(__import__("gzip").open(B / "reference-input-bundle.json.gz"))["resources"] if x["path"] == r["path"]); assert hashlib.sha256(data).hexdigest() == r["sha256"]
    (cwd / r["path"]).write_bytes(data)
out = W / f"out-{LABEL}"; out.mkdir(exist_ok=True)
rows = {}
for r in res["results"]:
    n = r["name"]
    src = pathlib.Path(r["source"]).read_bytes()
    assert hashlib.sha256(src).hexdigest() == r["source_sha256"], n
    svg = base64.b64decode(r["stdout_base64"]); assert hashlib.sha256(svg).hexdigest() == r["stdout_sha256"], n
    jtext = svg.decode("utf-8", "replace")
    (out / f"{n}.java.svg").write_bytes(svg)
    stderr = base64.b64decode(r["stderr_base64"]).decode("utf-8", "replace")
    row = dict(name=n, java_exit=r["exit_code"], java_stderr=stderr.strip()[:300],
               java_crash="An error has occured" in jtext, java_error_marker=any(m in jtext for m in ERRMARK) or not svg.strip(),
               intended_valid=man[n]["intended_valid"], axes=man[n]["axes"], source_sha256=r["source_sha256"],
               golden_sha256=r["stdout_sha256"])
    lines = []
    for b, tag in BINS:
        p = subprocess.run([b, "-pipe", "-tsvg"], input=src, capture_output=True, cwd=cwd, timeout=60,
                           env={"PATH": "/usr/bin:/bin:/opt/homebrew/bin", "RUSTUML_DEBUG": DEBUG, "TZ": "GMT+11:00", "PLANTUML_STDLIB": str(pathlib.Path.home() / "work/github.com/plantuml/plantuml-stdlib/stdlib")})
        (out / f"{n}.{tag}.svg").write_bytes(p.stdout)
        row[f"{tag}_exit"] = p.returncode
        row[f"{tag}_stderr"] = p.stderr.decode("utf-8", "replace").strip()[:300]
        row[f"{tag}_panic"] = b"panicked at" in p.stderr
        row[f"{tag}_comments_exact"] = C.findall(p.stdout.decode("utf-8", "replace")) == C.findall(jtext)
        row[f"{tag}_sha256"] = hashlib.sha256(p.stdout).hexdigest()
        lines.append(f"{out / (n + '.java.svg')}\t{out / (n + '.' + tag + '.svg')}\n")
    lst = W / f"lst-{LABEL}-{n}.txt"; lst.write_text("".join(lines))
    o = subprocess.run([str(CMP), str(lst)], capture_output=True, text=True).stdout.strip().split("\n")
    for (b, tag), line in zip(BINS, o):
        f = line.split("\t")
        row[f"{tag}_strict"] = f[1]; row[f"{tag}_ndiff"] = int(f[2]); row[f"{tag}_diff_head"] = f[3] if len(f) > 3 else ""
    rows[n] = row
(W / f"batch-{LABEL}.json").write_text(json.dumps(rows, indent=1, ensure_ascii=False))
tags = [t for _, t in BINS]
for n, r in rows.items():
    print(f"{n:34s} J{r['java_exit']:<3} " + " ".join(f"{t}:{r[t+'_exit']}/{r[t+'_strict'][:2]}{r[t+'_ndiff']}/{'c' if r[t+'_comments_exact'] else '-'}" for t in tags))
for t in tags:
    print(t, "strict", sum(r[t + "_strict"] == "MATCH" for r in rows.values()), "/", len(rows),
          "comments", sum(r[t + "_comments_exact"] for r in rows.values()))
