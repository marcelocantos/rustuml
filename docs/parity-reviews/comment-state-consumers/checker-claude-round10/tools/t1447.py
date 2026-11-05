"""T14.47 measurement: how many goldens / perturbations / standing passes depend on the state.rs arial_text_width "A"/"B" branch.
Runs base/tip and their v1 (branch removed, SansSerif fallback kept) and v2 (Arial table via measure_with_family) binaries over
every golden and perturbation whose source can select Arial (case-insensitive 'arial', or !theme/!include/!import), each from its
recorded cwd, and compares whole output with the unchanged strict comparator plus exact comment list.
Usage: python3 -I t1447.py <repo> <cwd_map.json> <out.json>"""
import json, os, pathlib, re, subprocess, sys, concurrent.futures, hashlib
REPO = pathlib.Path(sys.argv[1]); cmap = json.load(open(sys.argv[2])); OUT = pathlib.Path(sys.argv[3])
C = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round10-claude-checker"; CMP = C / "cmp/target/release/checker-cmp"
BINS = {t: C / f"bin/rustuml-{t}" for t in ["95291dc6b", "95291dc6b-v1", "95291dc6b-v2", "e24240b35", "e24240b35-v1", "e24240b35-v2"]}
W = C / "work/t1447"; W.mkdir(parents=True, exist_ok=True)
SEL = re.compile(rb"arial|!theme|!include|!import", re.I); COM = re.compile(r"<!--(.*?)-->", re.S)
env = dict(os.environ, RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100", PLANTUML_STDLIB=str(pathlib.Path.home() / "work/github.com/plantuml/plantuml-stdlib/stdlib"))
srcs = sorted(p for r in ["test-diagrams/golden", "test-diagrams/perturbations"] for p in (REPO / r).rglob("*.puml") if p.with_suffix(".svg").exists() and SEL.search(p.read_bytes()))
def cwd_for(rel):
    c = cmap.get(rel); return REPO / pathlib.Path(rel).parent if c is None or c.startswith("/") else REPO / c
def run(src):
    rel = str(src.relative_to(REPO)); res = {}
    for t, b in BINS.items():
        p = subprocess.run([str(b), "-pipe", "-tsvg"], input=src.read_bytes(), capture_output=True, cwd=cwd_for(rel), timeout=60, env=env)
        d = (W / t / rel).with_suffix(".svg"); d.parent.mkdir(parents=True, exist_ok=True); d.write_bytes(p.stdout)
        res[t] = hashlib.sha256(p.stdout).hexdigest()
    return rel, res
with concurrent.futures.ThreadPoolExecutor(10) as pool: shas = dict(pool.map(run, srcs))
lst = W / "cmp.lst"
with open(lst, "w") as f:
    for rel in shas:
        for t in BINS: f.write(f"{(REPO / rel).with_suffix('.svg')}\t{(W / t / rel).with_suffix('.svg')}\n")
cmp = {}
for line in subprocess.run([str(CMP), str(lst)], capture_output=True, text=True).stdout.splitlines():
    a, r, *_ = line.split("\t")
    for t in BINS:
        if f"/t1447/{t}/" in a: cmp[(t, a.split(f"/t1447/{t}/", 1)[1][:-4] + ".puml")] = r
def ok(t, rel):
    j = COM.findall((REPO / rel).with_suffix(".svg").read_text(errors="replace")); a = COM.findall((W / t / rel).with_suffix(".svg").read_text(errors="replace"))
    return cmp[(t, rel)] == "MATCH" and a == j
out = {"selected_sources": len(shas), "selector": SEL.pattern.decode(), "bins": {t: hashlib.sha256(b.read_bytes()).hexdigest() for t, b in BINS.items()}}
for tree in ["95291dc6b", "e24240b35"]:
    for v in ["v1", "v2"]:
        vt = f"{tree}-{v}"
        changed = [r for r in shas if shas[r][tree] != shas[r][vt]]
        lost = [r for r in changed if ok(tree, r) and not ok(vt, r)]; gained = [r for r in changed if ok(vt, r) and not ok(tree, r)]
        out[vt] = {"outputs_changed": changed, "lost_matches": lost, "gained_matches": gained,
                   "lost_goldens": [r for r in lost if "/golden/" in r], "lost_perturbations": [r for r in lost if "/perturbations/" in r]}
        print(vt, "changed", len(changed), "lost", len(lost), "(goldens", len(out[vt]["lost_goldens"]), ") gained", len(gained))
OUT.write_text(json.dumps(out, indent=1))
