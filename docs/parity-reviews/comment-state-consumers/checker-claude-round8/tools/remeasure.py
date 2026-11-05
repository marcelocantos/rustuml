"""Re-measure registered pairs (list of {name, source, golden, cwd}) at base/merge/tip from their recorded cwd.
Usage: python3 -I remeasure.py <repo> <in review json> <out json>"""
import json, pathlib, re, subprocess, sys, hashlib
REPO = pathlib.Path(sys.argv[1]); W = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round8-claude-checker"
CMP = W / "cmp/target/release/checker-cmp"; C = re.compile(r"<!--(.*?)-->", re.S)
BINS = {"base_5d8b1f7c1": W / "bin/rustuml-5d8b1f7c1", "merge_d6e9ffd94": W / "bin/rustuml-d6e9ffd94", "tip_c3116f0b0": W / "bin/rustuml-c3116f0b0"}
STDLIB = str(pathlib.Path.home() / "work/github.com/plantuml/plantuml-stdlib/stdlib")
rows = []
for p in json.load(open(sys.argv[2]))["perturbations"]:
    src = (REPO / p["source"]).read_bytes(); gold = REPO / p["golden"]; jc = C.findall(gold.read_text(errors="replace"))
    row = {k: p[k] for k in ("name", "axes", "source", "golden", "cwd") if k in p}; row["batch"] = "re-measured at round 8 (" + p.get("batch", "") + ")"
    tmp = W / "work/remeasure"; tmp.mkdir(exist_ok=True)
    for tag, b in BINS.items():
        r = subprocess.run([str(b), "-pipe", "-tsvg"], input=src, capture_output=True, cwd=REPO / p["cwd"], timeout=60,
                           env={"PATH": "/usr/bin:/bin:/opt/homebrew/bin", "RUSTUML_DEBUG": "date=1774210426000,tz=AEDT+1100", "TZ": "GMT+11:00", "PLANTUML_STDLIB": STDLIB})
        (tmp / f"{tag}.svg").write_bytes(r.stdout); (tmp / "l.txt").write_text(f"{gold}\t{tmp / (tag + '.svg')}\n")
        o = subprocess.run([str(CMP), str(tmp / "l.txt")], capture_output=True, text=True).stdout.split("\t")
        row[f"{tag}_strict_compare"] = o[1]; row[f"{tag}_comment_list_exact"] = C.findall(r.stdout.decode("utf-8", "replace")) == jc
    row["result"] = "pass" if row["tip_c3116f0b0_strict_compare"] == "MATCH" and row["tip_c3116f0b0_comment_list_exact"] else "fail"
    rows.append(row)
json.dump(rows, open(sys.argv[3], "w"), indent=1, ensure_ascii=False)
print(len(rows), "pass", sum(r["result"] == "pass" for r in rows), "merge-pass-tip-fail", [r["name"] for r in rows if r["merge_d6e9ffd94_strict_compare"] == "MATCH" and r["tip_c3116f0b0_strict_compare"] != "MATCH"])
