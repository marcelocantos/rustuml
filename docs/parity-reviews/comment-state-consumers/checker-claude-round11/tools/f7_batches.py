"""C3+C5 isolation on relay batches: f7c382ff4 (tables only) vs tip 97830bd1e, stdout bytes + exit, each case from its batch cwd.
Usage: python3 -I f7_batches.py <work> <out.json> L=<relaydir> ..."""
import base64, gzip, hashlib, json, pathlib, subprocess, sys
W = pathlib.Path(sys.argv[1]); B = pathlib.Path.home() / "Library/Caches/rustuml-shared-svg-round11-claude-checker/bin"
STDLIB = str(pathlib.Path.home() / "work/github.com/plantuml/plantuml-stdlib/stdlib"); res = {}
for a in sys.argv[3:]:
    L, d = a.split("=", 1); d = pathlib.Path(d)
    man = {c["name"]: c for c in json.load(open(d / "reference-manifest.json"))["cases"]}
    files = {pathlib.Path(f["path"]).stem: base64.b64decode(f["base64"]) for f in json.load(gzip.open(d / "reference-input-bundle.json.gz"))["files"]}
    diff = []
    for n, src in files.items():
        fmt = man[n]["invocation"][-1]; outs = []
        for b in ["rustuml-f7c382ff4", "rustuml-97830bd1e"]:
            p = subprocess.run([str(B / b), "-pipe", fmt], input=src, capture_output=True, cwd=W / f"cwd-{L}", timeout=60,
                               env={"PATH": "/usr/bin:/bin:/opt/homebrew/bin", "RUSTUML_DEBUG": "date=1774210426000,tz=AEDT+1100", "TZ": "GMT+11:00", "PLANTUML_STDLIB": STDLIB})
            outs.append((p.returncode, hashlib.sha256(p.stdout).hexdigest() if fmt != "-tpdf" else len(p.stdout)))
        if outs[0] != outs[1]: diff.append(n)
    res[L] = {"cases": len(files), "differ": diff}; print(L, len(files), "differ", diff)
json.dump(res, open(sys.argv[2], "w"), indent=1)
