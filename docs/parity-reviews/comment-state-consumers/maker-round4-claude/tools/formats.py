"""Run every output mode for carrier probes; scan for carrier/escape leaks.
Usage: formats.py <bin> <out.json> <src>..."""
import json, re, subprocess, sys, pathlib, hashlib
binary, out, *srcs = sys.argv[1:]
LEAK_RAW = re.compile("[\U0010F7FF-\U0010FFFF]")
LEAK_ESC = re.compile(r"\\u\{10f[7-9a-f][0-9a-f]{2}\}|&#(11120[6-9][0-9]|1112[1-9][0-9]{2}|111[3-4][0-9]{3});", re.I)
res = {}
for s in srcs:
    p = pathlib.Path(s); row = {}
    for mode in ["-tsvg", "-tpng", "-tpdf", "-teps", "-ttxt", "--ast", "--yaml"]:
        r = subprocess.run([binary, "-pipe", mode], input=p.read_bytes(), capture_output=True, cwd=p.parent, timeout=60,
                           env={"PATH": "/usr/bin:/bin", "RUSTUML_DEBUG": "date=1774210426000,tz=AEDT+1100"})
        text = r.stdout.decode("utf-8", "replace") if mode in ("-tsvg", "-ttxt", "--ast", "--yaml") else ""
        row[mode] = {"exit": r.returncode, "stderr": r.stderr.decode("utf-8", "replace").strip()[:160],
                     "stdout_bytes": len(r.stdout), "stdout_sha256": hashlib.sha256(r.stdout).hexdigest(),
                     "raw_reserved": len(LEAK_RAW.findall(text)), "escaped_reserved": len(LEAK_ESC.findall(text)),
                     "replacement_chars": text.count("\ufffd")}
    res[p.name] = row
json.dump(res, open(out, "w"), indent=1, ensure_ascii=False)
for n, row in res.items():
    print(f"{n:34s}", " ".join(f"{m}:{v['exit']}{'L' if v['raw_reserved'] or v['escaped_reserved'] else ''}" for m, v in row.items()),
          "|", next((v['stderr'][:70] for v in row.values() if v['exit']), ""))
