"""Held-out check: no probe source (by sha256 or by normalised text) appears in the repo's test-diagrams (goldens and
perturbations), any relay-parent bundle, or any other rustuml-* cache (checker and maker caches of every round).
Usage: python3 -I heldout_check_r5b.py PROBE_DIR OUT_JSON"""
import base64, gzip, hashlib, json, os, sys
from pathlib import Path
probe, out = Path(sys.argv[1]), Path(sys.argv[2])
H = Path.home() / "Library/Caches"
roots = [Path("/Users/marcelo/.t3/worktrees/rustuml/t3-18babc2b/test-diagrams")] + sorted(
    p for p in H.glob("rustuml-*") if p.is_dir() and p.name != "rustuml-opus-tim-r5-recheck")
sha = lambda b: hashlib.sha256(b).hexdigest(); norm = lambda b: b"\n".join(l.strip() for l in b.replace(b"\r", b"").split(b"\n") if l.strip())
seen_sha, seen_norm, n = set(), set(), 0
def add(b):
    global n; n += 1; seen_sha.add(sha(b)); seen_norm.add(sha(norm(b)))
for r in roots:
    for dp, dn, fn in os.walk(r):
        dn[:] = [d for d in dn if not d.startswith(("target", ".git")) and d not in ("deps", "build", "incremental")]
        for f in fn:
            p = os.path.join(dp, f)
            if f.endswith((".puml", ".iuml", ".txt", ".wsd")) and os.path.getsize(p) < 2_000_000: add(Path(p).read_bytes())
            elif f.endswith("input-bundle.json.gz"):
                try: d = json.loads(gzip.open(p).read())
                except Exception: continue
                for x in (d.get("files", []) + d.get("resources", [])) if isinstance(d, dict) else []:
                    if isinstance(x, dict) and "base64" in x: add(base64.b64decode(x["base64"]))
hits = []
for p in sorted(probe.iterdir()):
    if p.suffix not in (".puml", ".iuml"): continue
    b = p.read_bytes()
    if sha(b) in seen_sha or sha(norm(b)) in seen_norm: hits.append(p.name)
out.write_text(json.dumps(dict(roots=[str(r) for r in roots], corpus_files=n, probe_files=len([p for p in probe.iterdir() if p.suffix in (".puml", ".iuml")]), hits=hits), indent=1))
print("corpus", n, "hits", hits)
