"""Register Java-valid SVG pairs of batches BE (r4a, held out) and BC (c3a, not held out) per mechanism and
measure each registered pair from its registered cwd with the three private builds.

Usage: python3 -I register.py <repo> <out pairs.json>
Java-valid -tsvg cases become executable pairs under test-diagrams/perturbations/<mech>/checker-claude-round4/.
Evidence only: Java crash pages (parity_perturbations fails every preserved Java error page by design),
non-SVG formats (-tpng/-teps/-ttxt have no .svg golden), and -tpdf (pinned Java cannot produce PDF).
"""
import base64, gzip, hashlib, json, pathlib, re, subprocess, sys

REPO = pathlib.Path(sys.argv[1]); OUT = pathlib.Path(sys.argv[2])
W = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round4-claude-checker"
RP = pathlib.Path.home() / "Library/Caches/rustuml-relay-parent"
CMP = W / "cmp/target/release/checker-cmp"
BINS = {"base_3992971a7": W / "bin/rustuml-3992971a7", "round3_71d16da4a": W / "bin/rustuml-71d16da4a",
        "tip_7ec2e5af8": W / "bin/rustuml-7ec2e5af8"}
C = re.compile(r"<!--(.*?)-->", re.S)
ERRMARK = ["Syntax Error", "Welcome to PlantUML", "An error has occured", "[From string", "(Assumed diagram type:"]
DEBUG = "date=1774210426000,tz=AEDT+1100"
SUB = "checker-claude-round4"
BATCHES = {
    "BE": (RP / "shared-svg-round4-r4a", "ed243f9a5ffa8f8c3c8e9706d51be405644d601477626c515c0a7a69d09f77c5",
           "BE (relay batch r4a, held out from the maker)"),
    "BC": (RP / "shared-svg-round3-c3a", "ea80774390cf6b2e87f573929478a5c924eb81881ac81b5b255d029eee1baa03",
           "BC (relay batch c3a, given to the maker before drafting; not held out)"),
}
PLAN = {
    "tim-builtin-value-boundaries": lambda n: n.startswith(("pua_", "fmt_")),
    "comment-state-consumers": lambda n: n.startswith(("bp_", "no_", "ex_", "en_", "nc_")),
}
table = {m: [] for m in PLAN}
for label, (B, sha, desc) in BATCHES.items():
    assert hashlib.sha256((B / "reference-results.json").read_bytes()).hexdigest() == sha, label
    res = json.load(open(B / "reference-results.json"))["results"]
    man = {c["name"]: c for c in json.load(open(B / "reference-manifest.json"))["cases"]}
    files = {pathlib.Path(f["path"]).stem: base64.b64decode(f["base64"]) for f in json.load(gzip.open(B / "reference-input-bundle.json.gz"))["files"]}
    for r in sorted(res, key=lambda r: r["name"]):
        n = r["name"]; mech = next((m for m, pick in PLAN.items() if pick(n)), None)
        assert mech, n
        fmt = man[n]["invocation"][-1]
        src = files[n]; assert hashlib.sha256(src).hexdigest() == r["source_sha256"]
        out = base64.b64decode(r["stdout_base64"]); assert hashlib.sha256(out).hexdigest() == r["stdout_sha256"]
        stderr = base64.b64decode(r["stderr_base64"]).decode("utf-8", "replace")
        jt = out.decode("utf-8", "replace")
        crash = re.search(r"An error has occured : ([^<]{0,240})", jt)
        java_error = str(r["exit_code"]) != "0" or (fmt == "-tsvg" and (any(m in jt for m in ERRMARK) or not out.strip()))
        row = {"name": n, "batch": label, "format": fmt, "axes": man[n]["axes"], "intended_valid": man[n]["intended_valid"],
               "reference_batch": desc, "java_exit": int(r["exit_code"]), "java_stderr_head": stderr.strip()[:240],
               "java_error_page": java_error, "java_crash_text": crash.group(1).replace("&#160;", " ") if crash else None,
               "source_sha256": r["source_sha256"], "golden_sha256": r["stdout_sha256"]}
        d = REPO / "test-diagrams/perturbations" / mech / SUB
        if java_error or fmt != "-tsvg":
            row["registered"] = False
            row["reason_not_registered"] = (
                "pinned Java cannot produce PDF (TitledDiagram.java:345 PDF route commented out; TextBlockExporter12026.createUGraphic default branch throws UnsupportedOperationException: PDF): reference unavailable" if fmt == "-tpdf"
                else "Java export-crash page with exit 0 (parity_perturbations fails every preserved Java error page); evidence only" if java_error
                else f"{fmt} output has no .svg golden; compared outside parity_perturbations (exit, stderr, size/bytes); evidence only")
            cwd = W / f"work/cwd-{label}"
        else:
            d.mkdir(parents=True, exist_ok=True)
            (d / f"{n}.puml").write_bytes(src); (d / f"{n}.svg").write_bytes(out)
            row["registered"] = True
            row["source"] = f"test-diagrams/perturbations/{mech}/{SUB}/{n}.puml"
            row["golden"] = f"test-diagrams/perturbations/{mech}/{SUB}/{n}.svg"
            row["cwd"] = f"test-diagrams/perturbations/{mech}/{SUB}"
            cwd = d
        tmp = W / "work/register-tmp"; tmp.mkdir(exist_ok=True)
        (tmp / "java.svg").write_bytes(out)
        jc = C.findall(jt)
        for tag, b in BINS.items():
            p = subprocess.run([str(b), "-pipe", fmt], input=src, capture_output=True, cwd=cwd, timeout=60,
                               env={"PATH": "/usr/bin:/bin:/opt/homebrew/bin", "RUSTUML_DEBUG": DEBUG, "TZ": "GMT+11:00"})
            row[f"{tag}_cli_exit"] = p.returncode
            row[f"{tag}_stderr_head"] = p.stderr.decode("utf-8", "replace").strip()[:200]
            row[f"{tag}_stdout_sha256"] = hashlib.sha256(p.stdout).hexdigest()
            if fmt == "-tsvg":
                (tmp / f"{tag}.svg").write_bytes(p.stdout)
                (tmp / "l.txt").write_text(f"{tmp / 'java.svg'}\t{tmp / (tag + '.svg')}\n")
                o = subprocess.run([str(CMP), str(tmp / "l.txt")], capture_output=True, text=True).stdout.split("\t")
                row[f"{tag}_strict_compare"] = "match" if o[1] == "MATCH" else o[1]
                row[f"{tag}_difference_count"] = int(o[2])
                row[f"{tag}_comment_list_exact"] = C.findall(p.stdout.decode("utf-8", "replace")) == jc
            elif fmt == "-ttxt":
                row[f"{tag}_strict_compare"] = "match" if p.stdout == out else "DIFF"
        if java_error and fmt == "-tsvg":
            tip_err = row["tip_7ec2e5af8_stderr_head"]
            row["tip_outcome_vs_java"] = ("structured error, Java message" if row["tip_7ec2e5af8_cli_exit"] != 0 and crash
                and tip_err.removeprefix("export error: ") in row["java_crash_text"]
                else "structured error" if row["tip_7ec2e5af8_cli_exit"] != 0 else "renders normally where Java crashes")
        if row["registered"]:
            row["result"] = "pass" if row["tip_7ec2e5af8_strict_compare"] == "match" else "fail"
        else:
            row["result"] = "evidence-only"
        table[mech].append(row)
OUT.write_text(json.dumps(table, indent=1, ensure_ascii=False))
for mech, rows in table.items():
    reg = [r for r in rows if r["registered"]]
    print(mech, "cases", len(rows), "registered", len(reg), "pass", sum(r["result"] == "pass" for r in reg))
    for r in rows:
        f = lambda t: f"{r[t+'_cli_exit']}/{r.get(t+'_strict_compare','-')}"
        print(f"   {r['batch']} {r['name']:34s} {r['format']:6s} reg={r['registered']!s:5} J{r['java_exit']} base {f('base_3992971a7')} r3 {f('round3_71d16da4a')} tip {f('tip_7ec2e5af8')} {r['result']} {r.get('tip_outcome_vs_java','')}")
