"""Run a relayed Java reference batch through RustUML builds and compare whole CLI output.

Usage: python3 -I batch.py <work> <batch_dir> <label> <bin>=<tag> [...]
Each case runs with the case's own -t flag from a private cwd holding the batch resources (PRIVATE_CWD).
-tsvg: unchanged strict comparator (rustuml_oracle::compare::compare_svg_strict) + exact comment lists.
other formats: exit status, stderr head, byte equality (-ttxt text equality).
Output: <work>/batch-<label>.json
"""
import base64, gzip, hashlib, json, pathlib, re, subprocess, sys

W = pathlib.Path(sys.argv[1]); B = pathlib.Path(sys.argv[2]); LABEL = sys.argv[3]
BINS = [a.split("=", 1) for a in sys.argv[4:]]
CMP = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round5-claude-maker/cmp/target/release/checker-cmp"
C = re.compile(r"<!--(.*?)-->", re.S)
DEBUG = "date=1774210426000,tz=AEDT+1100"
res = json.load(open(B / "reference-results.json"))
man = {c["name"]: c for c in json.load(open(B / "reference-manifest.json"))["cases"]}
cwd = W / f"cwd-{LABEL}"; cwd.mkdir(parents=True, exist_ok=True)
bundle = json.load(gzip.open(B / "reference-input-bundle.json.gz"))
for r in bundle.get("resources", []):
    data = base64.b64decode(r["base64"]); assert hashlib.sha256(data).hexdigest() == r["sha256"]
    p = cwd / r["path"]; p.parent.mkdir(parents=True, exist_ok=True); p.write_bytes(data)
out = W / f"out-{LABEL}"; out.mkdir(exist_ok=True)
ENV = {"PATH": "/usr/bin:/bin:/opt/homebrew/bin", "RUSTUML_DEBUG": DEBUG, "TZ": "GMT+11:00",
       "PLANTUML_STDLIB": str(pathlib.Path.home() / "work/github.com/plantuml/plantuml-stdlib/stdlib")}
rows = {}
for r in res["results"]:
    n = r["name"]; fmt = r["argv"][-1]
    src = pathlib.Path(r["source"]).read_bytes()
    assert hashlib.sha256(src).hexdigest() == r["source_sha256"], n
    jout = base64.b64decode(r["stdout_base64"]); assert hashlib.sha256(jout).hexdigest() == r["stdout_sha256"], n
    ext = fmt[2:]
    (out / f"{n}.java.{ext}").write_bytes(jout)
    jtext = jout.decode("utf-8", "replace")
    stderr = base64.b64decode(r["stderr_base64"]).decode("utf-8", "replace")
    row = dict(name=n, format=fmt, java_exit=r["exit_code"], java_stderr=stderr.strip()[:300],
               java_crash="An error has occured" in jtext, intended_valid=man[n]["intended_valid"],
               axes=man[n]["axes"], source_sha256=r["source_sha256"], golden_sha256=r["stdout_sha256"])
    lines = []
    for b, tag in BINS:
        p = subprocess.run([b, "-pipe", fmt], input=src, capture_output=True, cwd=cwd, timeout=120, env=ENV)
        (out / f"{n}.{tag}.{ext}").write_bytes(p.stdout)
        row[f"{tag}_exit"] = p.returncode
        row[f"{tag}_stderr"] = p.stderr.decode("utf-8", "replace").strip()[:300]
        row[f"{tag}_panic"] = b"panicked at" in p.stderr
        row[f"{tag}_sha256"] = hashlib.sha256(p.stdout).hexdigest()
        row[f"{tag}_bytes_equal"] = p.stdout == jout
        if fmt == "-tsvg":
            row[f"{tag}_comments_exact"] = C.findall(p.stdout.decode("utf-8", "replace")) == C.findall(jtext)
            lines.append(f"{out / (n + '.java.svg')}\t{out / (n + '.' + tag + '.svg')}\n")
    if fmt == "-tsvg":
        lst = W / f"lst-{LABEL}-{n}.txt"; lst.write_text("".join(lines))
        o = subprocess.run([str(CMP), str(lst)], capture_output=True, text=True).stdout.strip().split("\n")
        for (b, tag), line in zip(BINS, o):
            f = line.split("\t")
            row[f"{tag}_strict"] = f[1]; row[f"{tag}_ndiff"] = int(f[2]); row[f"{tag}_diff_head"] = f[3] if len(f) > 3 else ""
    else:
        for b, tag in BINS:
            row[f"{tag}_strict"] = "BYTES" if row[f"{tag}_bytes_equal"] else ("EXIT_EQ" if row[f"{tag}_exit"] == row["java_exit"] else "EXIT_NE")
            row[f"{tag}_ndiff"] = 0
    rows[n] = row
(W / f"batch-{LABEL}.json").write_text(json.dumps(rows, indent=1, ensure_ascii=False))
tags = [t for _, t in BINS]
for n, r in rows.items():
    print(f"{n:44s} {r['format']:6s} J{r['java_exit']:<3} " + " ".join(f"{t}:{r[t+'_exit']}/{r[t+'_strict'][:5]}{r[t+'_ndiff']}/{'c' if r.get(t+'_comments_exact') else '-'}" for t in tags))
for t in tags:
    svg = [r for r in rows.values() if r["format"] == "-tsvg"]
    print(t, "svg strict MATCH", sum(r[t + "_strict"] == "MATCH" for r in svg), "/", len(svg),
          "java-exit0 svg MATCH", sum(r[t + "_strict"] == "MATCH" for r in svg if r["java_exit"] == 0), "/", sum(r["java_exit"] == 0 for r in svg),
          "comments", sum(bool(r.get(t + "_comments_exact")) for r in svg))
