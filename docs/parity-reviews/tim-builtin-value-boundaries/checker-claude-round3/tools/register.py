"""Register BB (relay batch r3a) pairs per mechanism and compute per-pair results from the registered cwd.

Usage: python3 -I register.py <repo> <out pairs.json>
Java-valid BB cases become executable pairs under test-diagrams/perturbations/<mech>/checker-claude-round3/.
Java export-crash pages (exit 0 with 'An error has occured') stay as evidence only, because
parity_perturbations fails every preserved Java error page by design (as in the round-2 review).
"""
import base64, hashlib, json, pathlib, re, subprocess, sys

REPO = pathlib.Path(sys.argv[1]); OUT = pathlib.Path(sys.argv[2])
W = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round3-claude-checker"
BB = pathlib.Path.home() / "Library/Caches/rustuml-relay-parent/shared-svg-round3-r3a"
CMP = W / "cmp/target/release/checker-cmp"
BINS = {"base_6617e48b8": W / "bin/rustuml-6617e48b8", "round2_6c52f2cfd": W / "bin/rustuml-6c52f2cfd",
        "tip_e6f629944": W / "bin/rustuml-e6f629944"}
C = re.compile(r"<!--(.*?)-->", re.S)
ERRMARK = ["Syntax Error", "Welcome to PlantUML", "An error has occured", "[From string", "(Assumed diagram type:"]
DEBUG = "date=1774210426000,tz=AEDT+1100"
SUB = "checker-claude-round3"
PLAN = {
    "tim-builtin-value-boundaries": lambda n: n.startswith("sur_"),
    "comment-state-consumers": lambda n: n.startswith(("dot_", "img_", "note_")),
}
assert hashlib.sha256((BB / "reference-results.json").read_bytes()).hexdigest() == \
    "c0379c640071f97fa848d5cd497a3c87cfe7605c2309d7f59b117ca6de0184ef"
res = json.load(open(BB / "reference-results.json"))["results"]
man = {c["name"]: c for c in json.load(open(BB / "reference-manifest.json"))["cases"]}
table = {}
for mech, pick in PLAN.items():
    d = REPO / "test-diagrams/perturbations" / mech / SUB
    rows = []
    for r in sorted(res, key=lambda r: r["name"]):
        n = r["name"]
        if not pick(n):
            continue
        src = pathlib.Path(r["source"]).read_bytes()
        assert hashlib.sha256(src).hexdigest() == r["source_sha256"]
        svg = base64.b64decode(r["stdout_base64"])
        assert hashlib.sha256(svg).hexdigest() == r["stdout_sha256"]
        stderr = base64.b64decode(r["stderr_base64"]).decode("utf-8", "replace")
        jt = svg.decode("utf-8", "replace")
        crash = re.search(r"An error has occured : ([^<]{0,240})", jt)
        java_error = str(r["exit_code"]) != "0" or any(m in jt for m in ERRMARK) or not svg.strip()
        row = {"name": n, "axes": man[n]["axes"], "intended_valid": man[n]["intended_valid"],
               "reference_batch": "BB (relay batch r3a, held out from the maker)", "java_exit": int(r["exit_code"]),
               "java_stderr_head": stderr.strip()[:240], "java_error_page": java_error,
               "java_crash_text": crash.group(1).replace("&#160;", " ") if crash else None,
               "source_sha256": r["source_sha256"], "golden_sha256": r["stdout_sha256"]}
        if java_error:
            row["registered"] = False
            row["reason_not_registered"] = "Java export-crash page with exit 0 (parity_perturbations fails every preserved Java error page); kept as evidence in the BB reference copy"
            cwd = W / "work/cwd-BB"
        else:
            d.mkdir(parents=True, exist_ok=True)
            (d / f"{n}.puml").write_bytes(src); (d / f"{n}.svg").write_bytes(svg)
            row["registered"] = True
            row["source"] = f"test-diagrams/perturbations/{mech}/{SUB}/{n}.puml"
            row["golden"] = f"test-diagrams/perturbations/{mech}/{SUB}/{n}.svg"
            row["cwd"] = f"test-diagrams/perturbations/{mech}/{SUB}"
            cwd = d
        tmp = W / "work/register-tmp"; tmp.mkdir(exist_ok=True)
        (tmp / "java.svg").write_bytes(svg)
        jc = C.findall(jt)
        for tag, b in BINS.items():
            p = subprocess.run([str(b), "-pipe", "-tsvg"], input=src, capture_output=True, cwd=cwd, timeout=60,
                               env={"PATH": "/usr/bin:/bin:/opt/homebrew/bin", "RUSTUML_DEBUG": DEBUG, "TZ": "GMT+11:00"})
            (tmp / f"{tag}.svg").write_bytes(p.stdout)
            (tmp / "l.txt").write_text(f"{tmp / 'java.svg'}\t{tmp / (tag + '.svg')}\n")
            o = subprocess.run([str(CMP), str(tmp / "l.txt")], capture_output=True, text=True).stdout.split("\t")
            rc = C.findall(p.stdout.decode("utf-8", "replace"))
            row[f"{tag}_cli_exit"] = p.returncode
            row[f"{tag}_stderr_head"] = p.stderr.decode("utf-8", "replace").strip()[:200]
            row[f"{tag}_stdout_sha256"] = hashlib.sha256(p.stdout).hexdigest()
            row[f"{tag}_strict_compare"] = "match" if o[1] == "MATCH" else o[1]
            row[f"{tag}_difference_count"] = int(o[2])
            row[f"{tag}_comment_list_exact"] = rc == jc
        if java_error:
            tip_err = row["tip_e6f629944_stderr_head"]
            row["tip_outcome_vs_java"] = ("structured error, Java message" if row["tip_e6f629944_cli_exit"] != 0
                and crash and tip_err.removeprefix("export error: ") in row["java_crash_text"]
                else "structured error" if row["tip_e6f629944_cli_exit"] != 0 else "renders normally where Java crashes")
        row["result"] = "pass" if (not java_error and row["tip_e6f629944_strict_compare"] == "match") else "fail"
        rows.append(row)
    table[mech] = rows
OUT.write_text(json.dumps(table, indent=1, ensure_ascii=False))
for mech, rows in table.items():
    reg = [r for r in rows if r["registered"]]
    print(mech, "cases", len(rows), "registered", len(reg), "pass", sum(r["result"] == "pass" for r in reg))
    for r in rows:
        print(f"   {r['name']:32s} reg={r['registered']!s:5} J{r['java_exit']} base {r['base_6617e48b8_cli_exit']}/{r['base_6617e48b8_strict_compare']} r2 {r['round2_6c52f2cfd_cli_exit']}/{r['round2_6c52f2cfd_strict_compare']} tip {r['tip_e6f629944_cli_exit']}/{r['tip_e6f629944_strict_compare']}/{r['tip_e6f629944_difference_count']} {r['result']} {r.get('tip_outcome_vs_java','')}")
