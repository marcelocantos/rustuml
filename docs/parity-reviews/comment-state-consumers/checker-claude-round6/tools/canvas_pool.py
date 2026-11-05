"""Generate canvas candidates and classify integer extents by base(ceil) vs tip((int)(x+1)) viewBox. Usage: python3 -I canvas_pool.py <pool_dir> <base_bin> <tip_bin>"""
import itertools, json, pathlib, re, subprocess, sys
P = pathlib.Path(sys.argv[1]); base, tip = sys.argv[2], sys.argv[3]
words = ["Pier", "Dock", "Quay", "Wharf", "Mooring", "Slipway", "Jetty", "Harbour", "Ko", "Lighthouse", "Buoy", "Anchorage", "Mm", "Il"]
T = {
 "outer": lambda a, b, c: f"@startuml\n[*] --> {a}\nstate {a} {{\n  [*] --> {b}\n  {b} --> {c}\n  {c} --> [*]\n}}\n{a} --> [*]\n@enduml\n",
 "root": lambda a, b, c: f"@startuml\nstate {a} {{\n  {b} --> {c}\n  note right of {c} : calm\n}}\nnote left of Gull : drift\nGull --> {b}\n@enduml\n",
 "rootlate": lambda a, b, c: f"@startuml\nnote left of {c} : busy\nstate {a} {{\n  {b} --> {c}\n}}\n@enduml\n",
 "flat": lambda a, b, c: f"@startuml\n[*] --> {a}\n{a} --> {b} : go\n{b} --> {c}\n{c} --> [*]\n@enduml\n",
}
rows = []
for kind, f in T.items():
    for a, b, c in itertools.islice(itertools.permutations(words, 3), 0, 2184, 7):
        src = f(a, b, c).encode(); vb = {}
        for tag, binp in (("base", base), ("tip", tip)):
            out = subprocess.run([binp, "-pipe", "-tsvg"], input=src, capture_output=True, timeout=30,
                                 env={"PATH": "/usr/bin:/bin:/opt/homebrew/bin", "RUSTUML_DEBUG": "date=1774210426000,tz=AEDT+1100"}).stdout.decode()
            m = re.search(r'viewBox="0 0 (\d+) (\d+)"', out); vb[tag] = m.groups() if m else None
        rows.append({"kind": kind, "names": [a, b, c], "base": vb["base"], "tip": vb["tip"], "src": src.decode()})
json.dump(rows, open(P / "pool.json", "w"), indent=0)
import collections
for kind in T:
    rs = [r for r in rows if r["kind"] == kind]
    print(kind, len(rs), "w-int", sum(r["base"] and r["tip"] and r["base"][0] != r["tip"][0] for r in rs), "h-int", sum(r["base"] and r["tip"] and r["base"][1] != r["tip"][1] for r in rs), "none", sum(not r["tip"] for r in rs))
