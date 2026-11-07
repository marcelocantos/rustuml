"""Exploration only (no Java): run each NAME.puml in DIR (cwd DIR) at the four builds and print texts.
Usage: python3 -I explore.py DIR [names...]"""
import os, re, subprocess, sys, hashlib
from pathlib import Path
d = Path(sys.argv[1]).resolve(); V = Path.home() / "Library/Caches/rustuml-tim-continuation-round3-claude-checker"
B = ["d28dd64a9", "a5720e095", "93bf53e18", "9834a7194"]
env = dict(os.environ, RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100",
           PLANTUML_STDLIB="/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib")
names = sys.argv[2:] or sorted(p.stem for p in d.glob("*.puml"))
for n in names:
    body = (d / f"{n}.puml").read_bytes(); outs = []
    for b in B:
        p = subprocess.run([str(V / "bin" / f"rustuml-{b}"), "-tsvg", "-pipe"], input=body, capture_output=True, cwd=d, env=env, timeout=60)
        svg = p.stdout.decode(errors="replace")
        t = [re.sub(r"<[^>]+>", "", x) for x in re.findall(r"<text[^>]*>[^<]*</text>", svg)]
        w = "WELCOME" if "Welcome to RustUML" in svg else ""
        outs.append((b[:4], p.returncode, hashlib.sha256(p.stdout).hexdigest()[:6], w, "|".join(t)[:90], p.stderr.decode(errors="replace").strip()[-80:]))
    print(f"== {n}")
    for o in outs: print("   ", *o)
