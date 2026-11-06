"""Re-verify the prepared fresh dirs (round-3 checker: BZ, CA, BY, BW added to REFS) against immutable Java reference results.
For every manifest row: source bytes and Java SVG bytes in the prepared dir must hash to the reference
results' source_sha256/stdout_sha256 for a case of the same name; validity is recomputed from raw SVG+stderr.
Usage: python3 -I verify_fresh.py OLD_CACHE OUT_JSON"""
import base64, gzip, hashlib, json, sys
from pathlib import Path
old, out = Path(sys.argv[1]), Path(sys.argv[2])
P = Path.home() / "Library/Caches/rustuml-relay-parent"
X = Path.home() / "Library/Caches/rustuml-sequence-sprite-checker-75ed0e64"
REFS = {"BZ": [P / "tim-continuation-mr3"], "CA": [P / "tim-continuation-cr3"], "BY": [P / "tim-continuation-cr2"], "BW": [P / "tim-continuation-r2"], "BU": [P / "tim-continuation-tc1"], "BD": [P / "tim-continuation-mc1"], "BA": [P / "sprite-round4-ck4"], "AY": [P / "sprite-round4-mk4"],
        "AZ": [P / "sprite-round4-mk4b"], "AX": [P / "sprite-round3-ck3"], "AV": [P / "sprite-round3-mk3"], "AT": [P / "sprite-ownership-checker-batch1"],
        "AS": [P / "sprite-claude-maker-batch2"], "AQ": [P / "sprite-claude-batch1"], "F790": [X], "F28": [X / "supplement"]}
sha = lambda b: hashlib.sha256(b).hexdigest()
# Validity: syntax/error markers in raw SVG+stderr (prepare.py list) plus the no-block and crash pages the brief names invalid.
MARKERS = ["Syntax Error", "NoSuchElementException", "Welcome to PlantUML", "An error has occured", "Cannot read",
           "No sprite defined", "graylevel are allowed", "[From string", "(Assumed diagram type:", "This feature has been suppressed",
           "No valid @start/@end found", "java.lang.", "Exception"]
def refs(cohort):
    idx = {}
    for d in REFS[cohort]:
        for f in [d / "reference-results.json"]:
            if not f.exists(): continue
            for r in json.loads(f.read_text())["results"]: idx[r["name"]] = r
    return idx
report = {}
for man in sorted(old.glob("fresh*-manifest.json")):
    if "raw" in man.name: continue
    rows = json.loads(man.read_text()); d = old / man.name.replace("-manifest.json", "")
    bad, valid_flip = [], []
    idxs = {}
    for row in rows:
        co = row["cohort"]; idx = idxs.setdefault(co, refs(co)); r = idx.get(row["name"])
        if r is None: bad.append((row["name"], "no reference")); continue
        src = (d / row["source"]).read_bytes(); svg = (d / row["golden"]).read_bytes()
        if sha(src) != r["source_sha256"]: bad.append((row["name"], "source sha"))
        if sha(svg) != r["stdout_sha256"]: bad.append((row["name"], "svg sha"))
        text = svg.decode(errors="replace") + "\n" + base64.b64decode(r["stderr_base64"]).decode(errors="replace")
        v = not any(m in text for m in MARKERS)
        if v != row["observed_valid"]: valid_flip.append(dict(name=row["name"], manifest=row["observed_valid"], strict=v, exit=r["exit_code"], hits=[m for m in MARKERS if m in text]))
    report[man.name] = dict(n=len(rows), bad=bad, validity_differs_under_strict_markers=valid_flip)
    print(man.name, len(rows), "bad", len(bad), "validity-diffs", len(valid_flip))
out.write_text(json.dumps(report, indent=1))
