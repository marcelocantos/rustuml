"""Render a manifest's cases with one or more binaries; write svg/err/exit per case.
usage: render.py <manifest.json> <outdir> <label=binary>... [--cwd DIR]"""
import hashlib, json, os, subprocess, sys
args = sys.argv[1:]
cwd = "/tmp"
if "--cwd" in args:
    i = args.index("--cwd"); cwd = args[i + 1]; del args[i:i + 2]
manifest, outdir, bins = args[0], args[1], args[2:]
m = json.load(open(manifest))
res = {}
for spec in bins:
    label, binary = spec.split("=", 1)
    d = os.path.join(outdir, label); os.makedirs(d, exist_ok=True)
    for c in m["cases"]:
        src = open(c["source"], "rb").read()
        assert hashlib.sha256(src).hexdigest() == c["source_sha256"], c["name"]
        try:
            p = subprocess.run([binary, "-pipe", "-tsvg"], input=src, capture_output=True, cwd=cwd, timeout=120)
            out, err, code = p.stdout, p.stderr, p.returncode
        except subprocess.TimeoutExpired:
            out, err, code = b"", b"TIMEOUT", -999
        open(os.path.join(d, c["name"] + ".svg"), "wb").write(out)
        res.setdefault(c["name"], {})[label] = {"exit": code, "stdout_sha256": hashlib.sha256(out).hexdigest(),
            "stderr_sha256": hashlib.sha256(err).hexdigest(), "stdout_bytes": len(out), "stderr_bytes": len(err)}
json.dump(res, open(os.path.join(outdir, "render.json"), "w"), indent=1, sort_keys=True)
print(len(res), "cases rendered")
