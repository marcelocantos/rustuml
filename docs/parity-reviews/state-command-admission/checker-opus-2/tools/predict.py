import hashlib, json, os, subprocess, sys
src_dir, out_dir, out_json = sys.argv[1:4]
trees = dict(a.split("=", 1) for a in sys.argv[4:])
os.makedirs(out_dir, exist_ok=True)
rep = {"trees": {t: {"binary": b, "sha256": hashlib.sha256(open(b, "rb").read()).hexdigest()} for t, b in trees.items()}, "cases": []}
for f in sorted(os.listdir(src_dir)):
    if not f.endswith(".puml"): continue
    name = f[:-5]; src = open(os.path.join(src_dir, f), "rb").read()
    case = {"name": name, "source_sha256": hashlib.sha256(src).hexdigest()}
    for t, b in trees.items():
        try:
            p = subprocess.run([b, "-pipe", "-tsvg"], input=src, capture_output=True, timeout=20)
        except subprocess.TimeoutExpired:
            case[t] = {"exit": 124, "stderr": "killed after 20 s"}; continue
        open(os.path.join(out_dir, f"{name}.{t}.svg"), "wb").write(p.stdout)
        case[t] = {"exit": p.returncode, "stdout_sha256": hashlib.sha256(p.stdout).hexdigest(), "stderr": p.stderr.decode("utf-8", "replace").strip()[:300]}
    rep["cases"].append(case)
json.dump(rep, open(out_json, "w"), indent=1)
for c in rep["cases"]:
    row = [c["name"]]
    for t in trees:
        e = c[t]; row.append(f'{t}:{e["exit"]}:{e.get("stdout_sha256","")[:8]}:{e["stderr"][:70]}')
    print(" | ".join(row))
