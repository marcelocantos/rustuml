"""Hand probe: run each build on one stdin (python escape string) and summarise. Usage: try.py 'SRC' [flags]"""
import os, re, subprocess, sys
V = os.path.expanduser("~/Library/Caches/rustuml-opus-tim-r7-recheck")
src = sys.argv[1].encode().decode("unicode_escape").encode("utf-8", "surrogatepass")
flags = sys.argv[2:] or ["-tsvg", "-pipe"]
env = dict(os.environ, RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100", PLANTUML_STDLIB="/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib")
for b in ["f6a738af1", "9ed52f435", "6d195c178", "df078f7d7"]:
    p = subprocess.run([f"{V}/bin/rustuml-{b}", *flags], input=src, capture_output=True, env=env, cwd=os.environ.get("PCWD", "/tmp"))
    out = p.stdout.decode(errors="replace")
    lines = re.findall(r'data-source-line="(\d+)"', out)
    texts = re.findall(r"<text[^>]*>([^<]*)</text>", out)
    dt = re.findall(r'data-diagram-type="([^"]*)"', out)
    print(f"{b} exit={p.returncode} type={dt[:1]} lines={lines[:12]} texts={texts[:10]} err={p.stderr.decode(errors='replace').strip()[-160:]!r}")
