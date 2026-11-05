"""Run relay cohorts directly: each case from a private cwd holding its source and resources.

Validity comes from the raw Java SVG and stderr (crash SVGs exit 0). Strict compare with the
unchanged comparator; exact comment lists. Usage:
python3 -I cohort_direct.py <relay-dir> <work> <out.json> <label=binary>...
"""
import base64, gzip, hashlib, json, os, pathlib, re, subprocess, sys

MARK = ["Syntax Error", "An error has occured", "has crashed", "Welcome to PlantUML", "Cannot load theme", "No such theme"]
CMT = re.compile(r"<!--(.*?)-->", re.S)
relay, work, out, *bins = sys.argv[1:]
relay, work = pathlib.Path(relay), pathlib.Path(work)
res = json.load(open(relay / "reference-results.json"))
bundle = json.load(gzip.open(relay / "reference-input-bundle.json.gz"))
files = {f["path"]: base64.b64decode(f["base64"]) for f in bundle["files"]}
resources = [(r["path"], base64.b64decode(r["base64"])) for r in bundle.get("resources", [])]
CMP = os.path.expanduser("~/Library/Caches/rustuml-shared-svg-round3-claude-maker/checker-cmp")
rows = []
for r in res["results"]:
    name = r["name"]
    src = files[r.get("remote_name", name + ".puml")]
    java = base64.b64decode(r["stdout_base64"]); jerr = base64.b64decode(r.get("stderr_base64", ""))
    jtext = java.decode("utf-8", "replace")
    java_valid = r["exit_code"] == 0 and not any(m in jtext for m in MARK) and not jerr.strip()
    row = {"case": name, "java_exit": r["exit_code"], "java_valid": java_valid,
           "java_stdout_sha256": hashlib.sha256(java).hexdigest()}
    case_dir = work / name; case_dir.mkdir(parents=True, exist_ok=True)
    (case_dir / "java.svg").write_bytes(java)
    for label_bin in bins:
        label, binary = label_bin.split("=", 1)
        cwd = work / label / name / "cwd"; cwd.mkdir(parents=True, exist_ok=True)
        for path, data in resources:
            p = cwd / path; p.parent.mkdir(parents=True, exist_ok=True); p.write_bytes(data)
        p = subprocess.run([binary, "-pipe", "-tsvg"], input=src, capture_output=True, cwd=cwd, timeout=60,
                           env=dict(os.environ, RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100"))
        outsvg = case_dir / f"{label}.svg"; outsvg.write_bytes(p.stdout)
        lst = case_dir / f"{label}.list"; lst.write_text(f"{case_dir/'java.svg'}\t{outsvg}\n")
        cmp = subprocess.run([CMP, str(lst)], capture_output=True, text=True).stdout.split("\t")
        verdict = cmp[1] if len(cmp) > 2 else "ERR"
        row[label] = {"exit": p.returncode, "panic": b"panicked at" in p.stderr,
                      "stderr_head": p.stderr[:160].decode("utf-8", "replace"),
                      "stdout_sha256": hashlib.sha256(p.stdout).hexdigest(),
                      "strict": verdict, "differences": int(cmp[2]) if len(cmp) > 2 and cmp[2].isdigit() else None,
                      "comments_exact": CMT.findall(p.stdout.decode("utf-8", "replace")) == CMT.findall(jtext)}
    rows.append(row)
labels = [b.split("=", 1)[0] for b in bins]
summary = {"cases": len(rows), "java_valid": sum(r["java_valid"] for r in rows)}
for l in labels:
    summary[l] = {"strict_match_valid": sum(1 for r in rows if r["java_valid"] and r[l]["strict"] == "MATCH"),
                  "comments_exact_valid": sum(1 for r in rows if r["java_valid"] and r[l]["comments_exact"]),
                  "rejected_invalid": sum(1 for r in rows if not r["java_valid"] and r[l]["exit"] != 0),
                  "errors_on_valid": [r["case"] for r in rows if r["java_valid"] and r[l]["exit"] != 0],
                  "panics": [r["case"] for r in rows if r[l]["panic"]]}
json.dump({"summary": summary, "rows": rows}, open(out, "w"), indent=1)
print(json.dumps(summary, indent=1))
