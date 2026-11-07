"""Held-out check: no probe source (by sha256 or by normalised text) appears in the goldens, the repo's test-diagrams,
any relay-parent bundle/manifest source, or the round-5 maker caches. Usage: python3 -I heldout_check.py PROBE_DIR OUT_JSON"""
import base64, gzip, hashlib, json, os, sys
from pathlib import Path
probe, out = Path(sys.argv[1]), Path(sys.argv[2])
H = Path.home() / "Library/Caches"
roots = [Path("/Users/marcelo/.t3/worktrees/rustuml/t3-7758f731/test-diagrams"), H / "rustuml-opus-tim-r5", H / "rustuml-grok-tim-r5",
         H / "rustuml-sequence-sprite-checker-75ed0e64", H / "rustuml-tim-continuation-round4-claude-checker"]
sha = lambda b: hashlib.sha256(b).hexdigest(); norm = lambda b: b"\n".join(l.strip() for l in b.replace(b"\r", b"").split(b"\n") if l.strip())
seen_sha, seen_norm, n = set(), set(), 0
def add(b):
    global n; n += 1; seen_sha.add(sha(b)); seen_norm.add(sha(norm(b)))
for r in roots:
    for dp, dn, fn in os.walk(r):
        for f in fn:
            if f.endswith((".puml", ".iuml", ".txt", ".wsd")) and os.path.getsize(os.path.join(dp, f)) < 2_000_000: add(Path(dp, f).read_bytes())
for b in (H / "rustuml-relay-parent").glob("*/reference-input-bundle.json.gz"):
    try: d = json.loads(gzip.open(b).read())
    except Exception: continue
    for x in d.get("files", []) + d.get("resources", []): add(base64.b64decode(x["base64"]))
hits = []
for p in sorted(probe.iterdir()):
    if p.suffix not in (".puml", ".iuml"): continue
    b = p.read_bytes()
    if sha(b) in seen_sha or sha(norm(b)) in seen_norm: hits.append(p.name)
out.write_text(json.dumps(dict(corpus_files=n, probe_files=len([p for p in probe.iterdir() if p.suffix in (".puml", ".iuml")]), hits=hits), indent=1))
print("corpus", n, "hits", hits)
