"""Independent recorded-cwd map: every review/result record under docs/parity-reviews that names a
perturbation source with a cwd field. '.'/worktree root -> repo root; PRIVATE_CWD -> source dir.
Usage: python3 -I cwd_map.py <repo> <out.json>"""
import json, pathlib, sys
repo = pathlib.Path(sys.argv[1]); out = {}; conflicts = {}
def walk(o):
    if isinstance(o, dict):
        src = o.get("source")
        cwd = o.get("cwd") or o.get("recorded_cwd")
        if isinstance(src, str) and src.endswith(".puml") and isinstance(cwd, str):
            c = cwd
            if c in (".", "") or c.startswith("/Users/"):
                c = "." if (c in (".", "") or "worktrees" in c or c.rstrip("/").endswith("rustuml")) else c
            if c.startswith("PRIVATE_CWD") or "worker" in c:
                c = str(pathlib.Path(src).parent)
            if src in out and out[src] != c:
                conflicts.setdefault(src, set()).update([out[src], c])
            out[src] = c
        for v in o.values(): walk(v)
    elif isinstance(o, list):
        for v in o: walk(v)
for p in sorted((repo / "docs/parity-reviews").rglob("*.json")):
    try: walk(json.loads(p.read_text()))
    except Exception: pass
json.dump(out, open(sys.argv[2], "w"), indent=0, sort_keys=True)
print(len(out), "mapped;", len(conflicts), "conflicts")
for k, v in list(conflicts.items())[:20]: print("  CONFLICT", k, v)
