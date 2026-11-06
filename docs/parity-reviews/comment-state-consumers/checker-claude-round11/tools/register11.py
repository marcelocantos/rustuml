"""Register Java-valid -tsvg pairs of relay batches (BV = maker M11A held out from the maker; BX = checker round-11 C11A held-out) under
test-diagrams/perturbations/<mech>/checker-claude-round11/ and measure each pair from its registered cwd.
Usage: python3 -I register.py <repo> <out pairs.json> [<batchname>=<dir>=<results_sha256>=<description> ...]
Java crash pages (exit 200 or crash text) are evidence only: parity_perturbations fails every preserved Java error page."""
import base64, gzip, hashlib, json, pathlib, re, subprocess, sys
REPO = pathlib.Path(sys.argv[1]); OUT = pathlib.Path(sys.argv[2])
W = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round11-claude-checker"
CMP = W / "cmp/target/release/checker-cmp"
BINS = {"base_5ddf7733c": W / "bin/rustuml-5ddf7733c", "merge_879d166f4": W / "bin/rustuml-879d166f4", "tip_97830bd1e": W / "bin/rustuml-97830bd1e"}
C = re.compile(r"<!--(.*?)-->", re.S); ERRMARK = ["Syntax Error", "Welcome to PlantUML", "An error has occured", "[From string", "(Assumed diagram type:", "does&#160;not&#160;sound&#160;like", "does not sound like"]
DEBUG = "date=1774210426000,tz=AEDT+1100"; SUB = "checker-claude-round11"
STDLIB = str(pathlib.Path.home() / "work/github.com/plantuml/plantuml-stdlib/stdlib")
BATCHES = [a.split("=", 3) for a in sys.argv[3:]]
STATE = re.compile(r"^(m11a_\w*_stat_|c11a_(sx|fc_(stat|sdsc|tlnk)|sz\d+_stat|cr_stat|st_stat)_)")
mech_of = lambda n: "comment-state-consumers" if STATE.match(n) else "tim-builtin-value-boundaries"
table = {"comment-state-consumers": [], "tim-builtin-value-boundaries": []}
for label, B, sha, desc in BATCHES:
    B = pathlib.Path(B); res_path = B / "reference-results.json"
    assert hashlib.sha256(res_path.read_bytes()).hexdigest() == sha, label
    res = json.load(open(res_path))["results"]
    man = {c["name"]: c for c in json.load(open(B / "reference-manifest.json"))["cases"]}
    files = {pathlib.Path(f["path"]).stem: base64.b64decode(f["base64"]) for f in json.load(gzip.open(B / "reference-input-bundle.json.gz"))["files"]}
    for r in sorted(res, key=lambda r: r["name"]):
        n = r["name"]; mech = mech_of(n); fmt = man[n]["invocation"][-1]
        src = files[n]; assert hashlib.sha256(src).hexdigest() == r["source_sha256"]
        out = base64.b64decode(r["stdout_base64"]); assert hashlib.sha256(out).hexdigest() == r["stdout_sha256"]
        jt = out.decode("utf-8", "replace"); crash = re.search(r"An error has occured : ([^<]{0,240})", jt)
        java_error = str(r["exit_code"]) != "0" or (fmt == "-tsvg" and (any(m in jt for m in ERRMARK) or not out.strip()))
        row = {"name": n, "batch": label, "format": fmt, "axes": man[n]["axes"], "intended_valid": man[n]["intended_valid"],
               "reference_batch": desc, "java_exit": int(r["exit_code"]), "java_error_page": java_error,
               "java_crash_text": crash.group(1).replace("&#160;", " ") if crash else None,
               "source_sha256": r["source_sha256"], "golden_sha256": r["stdout_sha256"]}
        d = REPO / "test-diagrams/perturbations" / mech / SUB
        if java_error or fmt != "-tsvg":
            row["registered"] = False
            row["reason_not_registered"] = "Java error page (exit " + str(r["exit_code"]) + ("; " + jt[jt.find("Your"):][:60] if "sound" in jt else "") + "); parity_perturbations fails every preserved Java error page; evidence only"
            cwd = W / "work" / f"cwd-{label}"; cwd.mkdir(parents=True, exist_ok=True)
        else:
            d.mkdir(parents=True, exist_ok=True); (d / f"{n}.puml").write_bytes(src); (d / f"{n}.svg").write_bytes(out)
            row.update(registered=True, source=f"test-diagrams/perturbations/{mech}/{SUB}/{n}.puml",
                       golden=f"test-diagrams/perturbations/{mech}/{SUB}/{n}.svg", cwd=f"test-diagrams/perturbations/{mech}/{SUB}")
            cwd = d
        tmp = W / "work/register-tmp"; tmp.mkdir(exist_ok=True); (tmp / "java.svg").write_bytes(out); jc = C.findall(jt)
        for tag, b in BINS.items():
            p = subprocess.run([str(b), "-pipe", fmt], input=src, capture_output=True, cwd=cwd, timeout=60,
                               env={"PATH": "/usr/bin:/bin:/opt/homebrew/bin", "RUSTUML_DEBUG": DEBUG, "TZ": "GMT+11:00", "PLANTUML_STDLIB": STDLIB})
            row[f"{tag}_cli_exit"] = p.returncode; row[f"{tag}_stderr_head"] = p.stderr.decode("utf-8", "replace").strip()[:200]
            row[f"{tag}_stdout_sha256"] = hashlib.sha256(p.stdout).hexdigest()
            (tmp / f"{tag}.svg").write_bytes(p.stdout); (tmp / "l.txt").write_text(f"{tmp / 'java.svg'}\t{tmp / (tag + '.svg')}\n")
            o = subprocess.run([str(CMP), str(tmp / "l.txt")], capture_output=True, text=True).stdout.split("\t")
            row[f"{tag}_strict_compare"] = "match" if o[1] == "MATCH" else o[1]; row[f"{tag}_difference_count"] = int(o[2])
            row[f"{tag}_comment_list_exact"] = C.findall(p.stdout.decode("utf-8", "replace")) == jc
        if java_error:
            row["tip_outcome_vs_java"] = "structured error" if row["tip_97830bd1e_cli_exit"] != 0 else "renders normally where Java crashes"
            row["result"] = "evidence-only"
        else:
            row["result"] = "pass" if row["tip_97830bd1e_strict_compare"] == "match" and row["tip_97830bd1e_comment_list_exact"] else "fail"
        table[mech].append(row)
OUT.write_text(json.dumps(table, indent=1, ensure_ascii=False))
for mech, rows in table.items():
    reg = [r for r in rows if r["registered"]]
    print(mech, "cases", len(rows), "registered", len(reg), "pass", sum(r["result"] == "pass" for r in reg))
    for r in rows:
        if r["result"] != "pass":
            f = lambda t: f"{r[t+'_cli_exit']}/{r[t+'_strict_compare']}{r[t+'_difference_count']}"
            print(f"   {r['batch']} {r['name']:40s} J{r['java_exit']} base {f('base_5ddf7733c')} merge {f('merge_879d166f4')} tip {f('tip_97830bd1e')} {r['result']} {r.get('tip_outcome_vs_java','')}")
