"""Classify before/after sweep status and strict-compare events against the Java goldens.

Validity comes from the raw Java SVG (error markers), never from an exit code.
Usage: python3 -I classify.py <repo> <out-before> <out-after> <cmp.tsv> <events.json>
cmp.tsv lines: <actual svg path> TAB MATCH|DIFF|ERR TAB ... (checker-cmp output)
"""
import collections, json, pathlib, sys

MARK = ["Syntax Error", "NoSuchElementException", "Welcome to PlantUML", "An error has occured",
        "kill cannot be used here", "swimlane must be defined at the start", "Note already created:",
        "Parsing syntax error about %", "[From string", "Your data does not sound like YAML data",
        "does&#160;not&#160;sound&#160;like&#160;YAML", "Your data does not sound like JSON data",
        "does&#160;not&#160;sound&#160;like&#160;JSON", "No class ", "(Assumed diagram type:",
        "DITAA has crashed", "This feature has been suppressed"]
GET_PROVENANCE = {"test-diagrams/golden/activity/act_skinparam_shadowing.puml",
                  "test-diagrams/golden/activity/act_skinparam2_shadowing_true.puml",
                  "test-diagrams/golden/edge-cases/edge_whitespace_multiple_blank_lines.puml"}

repo, ob, oa, cmp_path, out = sys.argv[1:]
repo = pathlib.Path(repo)
st = {"before": json.load(open(pathlib.Path(ob) / "status.json")),
      "after": json.load(open(pathlib.Path(oa) / "status.json"))}
roots = {"before": str(pathlib.Path(ob).resolve()) + "/", "after": str(pathlib.Path(oa).resolve()) + "/"}
cmp = {}
for line in open(cmp_path):
    actual, verdict = line.rstrip("\n").split("\t")[:2]
    for side, root in roots.items():
        if actual.startswith(root):
            cmp[(side, actual[len(root):-4] + ".puml")] = verdict
events = collections.defaultdict(list)
for rel in st["after"]:
    java = (repo / rel).with_suffix(".svg").read_text(errors="replace")
    java_error = not java.strip() or any(m in java for m in MARK)
    b, a = st["before"].get(rel), st["after"][rel]
    if b is None:
        events["new_case"].append(rel)
        continue
    bs, as_ = str(b["status"]), str(a["status"])
    if bs == "0" and as_ != "0":
        events["rejection_gained" if java_error else "new_valid_input_error"].append(rel)
    if bs != "0" and as_ == "0":
        events["lost_rejection" if java_error else "error_now_renders_valid_ref"].append(rel)
    if a["panic"] and not b["panic"]:
        events["new_panic"].append(rel)
    if b["panic"] and not a["panic"]:
        events["panic_removed"].append(rel)
    if as_ == "timeout" and bs != "timeout":
        events["new_timeout"].append(rel)
    if not java_error:
        mb, ma = cmp.get(("before", rel)) == "MATCH", cmp.get(("after", rel)) == "MATCH"
        if mb and not ma:
            events["lost_match_get_provenance" if rel in GET_PROVENANCE else "lost_match"].append(rel)
        if ma and not mb:
            events["gained_match"].append(rel)
    if b["stdout_sha256"] != a["stdout_sha256"]:
        events["bytes_changed"].append(rel)
summary = {"sources": len(st["after"]),
           "matches_before": sum(1 for (s, _), v in cmp.items() if s == "before" and v == "MATCH"),
           "matches_after": sum(1 for (s, _), v in cmp.items() if s == "after" and v == "MATCH"),
           "events": {k: sorted(v) for k, v in sorted(events.items())}}
json.dump(summary, open(out, "w"), indent=1)
print("sources", summary["sources"], "matches", summary["matches_before"], "->", summary["matches_after"])
for kind, cases in sorted(events.items()):
    print(kind, len(cases))
    if kind != "bytes_changed" and kind != "new_case" and kind != "gained_match":
        for case in cases:
            print("   ", case, st["before"][case]["status"], "->", st["after"][case]["status"])
