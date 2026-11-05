"""Run a relay batch through each RustUML build with each case's own -t flag; whole-output comparison.

Usage: python3 -I batch.py <work> <batch_dir> <label> <bin>=<tag> [...]
SVG: unchanged strict comparator + exact comment list. TXT: byte equality. PNG: IHDR size + exit/stderr.
EPS: %%BoundingBox + exit/stderr. Java PDF is unavailable in the pinned environment (T14.6.5).
Each case runs from a private cwd (PRIVATE_CWD) holding the batch resources. Output: <work>/batch-<label>.json
"""
import base64, gzip, hashlib, json, pathlib, re, struct, subprocess, sys

W = pathlib.Path(sys.argv[1]); B = pathlib.Path(sys.argv[2]); LABEL = sys.argv[3]
BINS = [a.split("=", 1) for a in sys.argv[4:]]
CMP = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round6-claude-maker/cmp/target/release/checker-cmp"
C = re.compile(r"<!--(.*?)-->", re.S)
DEBUG = "date=1774210426000,tz=AEDT+1100"
ERRMARK = ["Syntax Error", "Welcome to PlantUML", "An error has occured", "[From string", "(Assumed diagram type:"]
STDLIB = str(pathlib.Path.home() / "work/github.com/plantuml/plantuml-stdlib/stdlib")

def png_size(b):
    return list(struct.unpack(">II", b[16:24])) if b[:8] == b"\x89PNG\r\n\x1a\n" else None
def eps_bbox(b):
    m = re.search(rb"%%BoundingBox: ([^\n]*)", b); return m.group(1).decode().strip() if m else None

res = json.load(open(B / "reference-results.json"))
man = {c["name"]: c for c in json.load(open(B / "reference-manifest.json"))["cases"]}
bundle = json.load(gzip.open(B / "reference-input-bundle.json.gz"))
cwd = W / f"cwd-{LABEL}"; cwd.mkdir(parents=True, exist_ok=True)
for r in bundle.get("resources", []):
    data = base64.b64decode(r["base64"]); assert hashlib.sha256(data).hexdigest() == r["sha256"]
    (cwd / r["path"]).parent.mkdir(parents=True, exist_ok=True); (cwd / r["path"]).write_bytes(data)
srcs = {pathlib.Path(f["path"]).stem: base64.b64decode(f["base64"]) for f in bundle["files"]}
out = W / f"out-{LABEL}"; out.mkdir(exist_ok=True)
rows = {}
for r in res["results"]:
    n = r["name"]; fmt = man[n]["invocation"][-1]; ext = fmt[2:]
    src = srcs[n]; assert hashlib.sha256(src).hexdigest() == r["source_sha256"], n
    jout = base64.b64decode(r["stdout_base64"]); assert hashlib.sha256(jout).hexdigest() == r["stdout_sha256"], n
    (out / f"{n}.java.{ext}").write_bytes(jout)
    jstderr = base64.b64decode(r["stderr_base64"]).decode("utf-8", "replace")
    jtext = jout.decode("utf-8", "replace")
    row = dict(name=n, format=fmt, java_exit=r["exit_code"], java_stderr=jstderr.strip()[:400],
               java_crash="An error has occured" in jtext,
               java_error_marker=(ext == "svg" and (any(m in jtext for m in ERRMARK) or not jout.strip())),
               intended_valid=man[n]["intended_valid"], axes=man[n]["axes"], source_sha256=r["source_sha256"],
               java_stdout_sha256=r["stdout_sha256"], java_stdout_bytes=len(jout))
    if ext == "png": row["java_png_size"] = png_size(jout)
    if ext == "eps": row["java_eps_bbox"] = eps_bbox(jout)
    lines = []
    for b, tag in BINS:
        p = subprocess.run([b, "-pipe", fmt], input=src, capture_output=True, cwd=cwd, timeout=60,
                           env={"PATH": "/usr/bin:/bin:/opt/homebrew/bin", "RUSTUML_DEBUG": DEBUG, "TZ": "GMT+11:00", "PLANTUML_STDLIB": STDLIB})
        (out / f"{n}.{tag}.{ext}").write_bytes(p.stdout)
        row[f"{tag}_exit"] = p.returncode
        row[f"{tag}_stderr"] = p.stderr.decode("utf-8", "replace").strip()[:400]
        row[f"{tag}_stderr_sha256"] = hashlib.sha256(p.stderr).hexdigest()
        row[f"{tag}_panic"] = b"panicked at" in p.stderr
        row[f"{tag}_sha256"] = hashlib.sha256(p.stdout).hexdigest()
        row[f"{tag}_bytes"] = len(p.stdout)
        if ext == "svg":
            row[f"{tag}_comments_exact"] = C.findall(p.stdout.decode("utf-8", "replace")) == C.findall(jtext)
            lines.append((tag, f"{out / (n + '.java.svg')}\t{out / (n + '.' + tag + '.svg')}\n"))
        elif ext == "txt":
            row[f"{tag}_strict"] = "MATCH" if p.stdout == jout else "DIFF"; row[f"{tag}_ndiff"] = 0 if p.stdout == jout else 1
        elif ext == "png":
            row[f"{tag}_png_size"] = png_size(p.stdout)
            row[f"{tag}_strict"] = "N/A-raster"; row[f"{tag}_ndiff"] = -1
        elif ext == "eps":
            row[f"{tag}_eps_bbox"] = eps_bbox(p.stdout)
            row[f"{tag}_strict"] = "N/A-eps"; row[f"{tag}_ndiff"] = -1
        else:
            row[f"{tag}_strict"] = "N/A-java-pdf-unavailable"; row[f"{tag}_ndiff"] = -1
            row[f"{tag}_pdf_magic"] = p.stdout[:5].decode("latin1")
    if lines:
        lst = W / f"lst-{LABEL}-{n}.txt"; lst.write_text("".join(l for _, l in lines))
        o = subprocess.run([str(CMP), str(lst)], capture_output=True, text=True).stdout.strip().split("\n")
        for (tag, _), line in zip(lines, o):
            f = line.split("\t")
            row[f"{tag}_strict"] = f[1]; row[f"{tag}_ndiff"] = int(f[2]); row[f"{tag}_diff_head"] = f[3] if len(f) > 3 else ""
    rows[n] = row
(W / f"batch-{LABEL}.json").write_text(json.dumps(rows, indent=1, ensure_ascii=False))
tags = [t for _, t in BINS]
for n, r in rows.items():
    print(f"{n:34s} {r['format']:6s} J{r['java_exit']:<2}" + " ".join(f"{t}:{r[t+'_exit']}/{r[t+'_strict'][:5]}{r[t+'_ndiff']}/{'c' if r.get(t+'_comments_exact') else '-'}" for t in tags))
for t in tags:
    print(t, "strict", sum(r[t + "_strict"] == "MATCH" for r in rows.values()), "/", len(rows))
