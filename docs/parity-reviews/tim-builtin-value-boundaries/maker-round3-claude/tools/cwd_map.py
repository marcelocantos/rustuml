"""Recorded-cwd map for every test-diagrams source cited by a parity review.

A source runs from the cwd its reference was generated with. Records give:
  '.' or a worktree root            -> repository root
  a path ending in test-diagrams/... -> that repository-relative directory
  PRIVATE_CWD / relay worker dirs    -> the reference staged the source (and
                                        any resources) alone, i.e. its own dir
Unrecorded sources (goldens included) run from their own directory.
Usage: python3 -I cwd_map.py <repo> <out.json>
"""
import glob, json, os, re, sys

repo, out = sys.argv[1], sys.argv[2]
os.chdir(repo)
records = {}

def walk(node, found):
    if isinstance(node, dict):
        if isinstance(node.get("source"), str) and isinstance(node.get("cwd"), str):
            found.append((node["source"], node["cwd"]))
        for value in node.values():
            walk(value, found)
    elif isinstance(node, list):
        for value in node:
            walk(value, found)

for path in sorted(glob.glob("docs/parity-reviews/**/*.json", recursive=True)):
    try:
        data = json.load(open(path))
    except (ValueError, UnicodeDecodeError):
        continue
    found = []
    walk(data, found)
    for source, cwd in found:
        if source.startswith("test-diagrams/") and source.endswith(".puml"):
            records.setdefault(source, set()).add(cwd)

def classify(source, cwd):
    if cwd in (".", "") or re.fullmatch(r"/.*/(t3-[0-9a-f]+|rustuml)/?", cwd):
        return "."
    match = re.search(r"(?:^|/)(test-diagrams/.*?)/?$", cwd)
    if match:
        return match.group(1)
    return os.path.dirname(source)

result = {}
for source, cwds in sorted(records.items()):
    kinds = {classify(source, cwd) for cwd in cwds}
    explicit = sorted(k for k in kinds if k != os.path.dirname(source) and k != ".")
    result[source] = explicit[0] if explicit else ("." if "." in kinds else os.path.dirname(source))
json.dump(result, open(out, "w"), indent=0, sort_keys=True)
print(len(result), "recorded sources;", sum(1 for v in result.values() if v == "."), "at repo root")
