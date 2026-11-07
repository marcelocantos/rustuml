"""Run the round-4 attribution controls on the three builds (Rust only; Java values come from AY/AX references or ck4).
Usage: python3 -I controls.py OUT_JSON"""
import hashlib, json, os, re, subprocess, sys
from pathlib import Path
C = Path(os.path.expanduser("~/Library/Caches/rustuml-sprite-round4-claude-checker"))
REPO = Path("/Users/marcelo/.t3/worktrees/rustuml/t3-388b0a83")
env = dict(os.environ, RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100", PLANTUML_STDLIB="/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib")
AY = (C / "fresh-ay/mk4_tim_sprite_in_procedure.puml").read_text()
CCO = (REPO / "test-diagrams/perturbations/tim-evaluated-event-stream/checker_comments_and_origins.puml").read_text()
SRC = {
 "mk4_tim_sprite_in_procedure (AY, Java data-source-line 8)": AY,
 "same, comment removed": AY.replace(" /' e '/", ""),
 "procedure emitting four classes, then class Oak (raw line 9)": "@startuml\n!procedure $mk()\nclass P1\nclass P2\nclass P3\nclass P4\n!endprocedure\n$mk()\nclass Oak\n@enduml\n",
 "seq continuation control: A -> B : one \\ / two": "@startuml\nA -> B : one \\\ntwo\n@enduml\n",
 "seq comment then backslash: /' c '/ \\ / A -> B : hi": "@startuml\n/' c '/ \\\nA -> B : hi\n@enduml\n",
 "checker_comments_and_origins": CCO,
 "checker_comments_and_origins, comments removed": CCO.replace("/' before '/ ", "").replace(" /' trailing '/", ""),
}
out = {}
for label, src in SRC.items():
    row = {}
    for b in ("93bf53e18", "044f73657", "c9e11493c"):
        p = subprocess.run([str(C / f"bin/rustuml-{b}"), "-tsvg", "-pipe"], input=src.encode(), capture_output=True, env=env, timeout=60)
        svg = p.stdout.decode(errors="replace")
        row[b] = dict(exit=p.returncode, svg_sha256=hashlib.sha256(p.stdout).hexdigest(),
                      svg_sha256_gradient_ids_normalised=hashlib.sha256(re.sub(r'g[0-9a-z]{10,16}(?=[01]\b|[01]")', "GID", svg).encode()).hexdigest(),
                      source_lines=re.findall(r'data-qualified-name="([^"]*)" data-source-line="(\d+)"', svg),
                      stderr=p.stderr.decode(errors="replace")[:300])
    out[label] = dict(source=src, results=row)
Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")
for k, v in out.items():
    print(k); [print("  ", b, r["exit"], r["source_lines"], r["svg_sha256_gradient_ids_normalised"][:10], r["stderr"][:90].replace("\n", " ")) for b, r in v["results"].items()]
