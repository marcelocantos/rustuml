"""Do the registered round-10 pairs (and the pre-existing perturbation corpus selected by 'FontStyle') catch the consumer-site
italic mutants that survive the lib suite? Runs each mutant CLI over every registered pair that passes at tip, from its
registered cwd, and reports pairs that stop matching. Usage: python3 -I mutant_net10.py <repo> <pairs.json> <out.json>"""
import json, os, pathlib, re, subprocess, sys, concurrent.futures, tempfile
REPO = pathlib.Path(sys.argv[1]); pairs = json.load(open(sys.argv[2])); OUT = pathlib.Path(sys.argv[3])
C = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round10-claude-checker"; CMP = C / "cmp/target/release/checker-cmp"
COM = re.compile(r"<!--(.*?)-->", re.S)
env = dict(os.environ, RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100", TZ="GMT+11:00", PLANTUML_STDLIB=str(pathlib.Path.home() / "work/github.com/plantuml/plantuml-stdlib/stdlib"))
rows = [r for rs in pairs.values() for r in rs if r["registered"] and r["result"] == "pass"]
muts = sorted(p.name[len("rustuml-mut-"):] for p in (C / "bin").glob("rustuml-mut-*"))
W = C / "work/mutnet"; W.mkdir(parents=True, exist_ok=True); res = {}
for m in muts:
    b = C / f"bin/rustuml-mut-{m}"; d = W / m; d.mkdir(exist_ok=True)
    def run(r):
        p = subprocess.run([str(b), "-pipe", "-tsvg"], input=(REPO / r["source"]).read_bytes(), capture_output=True, cwd=REPO / r["cwd"], env=env, timeout=60)
        (d / f"{r['name']}.svg").write_bytes(p.stdout); return r
    with concurrent.futures.ThreadPoolExecutor(10) as pool: list(pool.map(run, rows))
    lst = W / f"{m}.lst"; lst.write_text("".join(f"{REPO / r['golden']}\t{d / (r['name'] + '.svg')}\n" for r in rows))
    out = subprocess.run([str(CMP), str(lst)], capture_output=True, text=True).stdout.splitlines()
    caught = []
    for r, line in zip(rows, out):
        ok = line.split("\t")[1] == "MATCH" and COM.findall((d / f"{r['name']}.svg").read_text(errors="replace")) == COM.findall((REPO / r["golden"]).read_text(errors="replace"))
        if not ok: caught.append(r["name"])
    res[m] = {"pairs_checked": len(rows), "pairs_failing": len(caught), "caught": bool(caught), "examples": caught[:12]}
    print(m, len(caught), caught[:5], flush=True)
OUT.write_text(json.dumps(res, indent=1))
