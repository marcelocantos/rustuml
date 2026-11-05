"""Classify sweep events between two trees. Usage: classify.py <work> <repo> <before> <after> <out.json>"""
import json, sys, pathlib, collections, re
W, REPO, B, A, OUT = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3], sys.argv[4], sys.argv[5]
MARK = ["Syntax Error","NoSuchElementException","Welcome to PlantUML","An error has occured","kill cannot be used here",
 "swimlane must be defined at the start","Note already created:","Parsing syntax error about %","[From string",
 "Your data does not sound like YAML data","does&#160;not&#160;sound&#160;like&#160;YAML","Your data does not sound like JSON data",
 "does&#160;not&#160;sound&#160;like&#160;JSON","No class ","(Assumed diagram type:","DITAA has crashed","This feature has been suppressed"]
GET = {"test-diagrams/golden/activity/act_skinparam_shadowing.puml","test-diagrams/golden/activity/act_skinparam2_shadowing_true.puml",
       "test-diagrams/golden/edge-cases/edge_whitespace_multiple_blank_lines.puml"}
C = re.compile(r"<!--(.*?)-->", re.S)
st = {s: json.load(open(W / f"sweep-{s}/status.json")) for s in (B, A)}
cmp = {}
for line in open(W / "cmp.tsv"):
    a, r, *_ = line.rstrip("\n").split("\t")
    for s in (B, A):
        tag = f"/sweep-{s}/"
        if tag in a:
            cmp[(s, a.split(tag, 1)[1][:-4] + ".puml")] = r
ev = collections.defaultdict(list); cm = collections.defaultdict(list)
n_match = {B: 0, A: 0}; n_cexact = {B: 0, A: 0}
for k in sorted(st[A]):
    ref = (REPO / k).with_suffix(".svg").read_text(errors="replace")
    jerr = (len(ref.strip()) == 0) or any(m in ref for m in MARK)
    b, a = st[B][k], st[A][k]
    bs, as_ = str(b["status"]), str(a["status"])
    if bs == "0" and as_ != "0": ev["rejection_gained" if jerr else "new_valid_input_error"].append(k)
    if bs != "0" and as_ == "0": ev["lost_rejection" if jerr else "error_now_renders_valid_ref"].append(k)
    if a["panic"] and not b["panic"]: ev["new_panic"].append(k)
    if b["panic"] and not a["panic"]: ev["panic_removed"].append(k)
    if as_ == "timeout" and bs != "timeout": ev["new_timeout"].append(k)
    if bs == "timeout" and as_ != "timeout": ev["timeout_removed"].append(k)
    mb, ma = cmp[(B, k)] == "MATCH", cmp[(A, k)] == "MATCH"
    n_match[B] += mb and not jerr; n_match[A] += ma and not jerr
    if not jerr:
        if mb and not ma: ev["lost_match_GET_provenance" if k in GET else "lost_match"].append(k)
        if ma and not mb: ev["gained_match"].append(k)
    if b["stdout_sha256"] != a["stdout_sha256"]: ev["outputs_changed"].append(k)
    jc = C.findall(ref)
    eb = C.findall((W / f"sweep-{B}" / k).with_suffix(".svg").read_text(errors="replace")) == jc
    ea = C.findall((W / f"sweep-{A}" / k).with_suffix(".svg").read_text(errors="replace")) == jc
    n_cexact[B] += eb; n_cexact[A] += ea
    if eb and not ea: cm["lost"].append(k)
    if ea and not eb: cm["gained"].append(k)
res = {"before": B, "after": A, "sources": len(st[A]), "strict_valid_matches": n_match, "comment_exact": n_cexact,
       "events": {k: sorted(v) for k, v in ev.items()}, "comments": {k: sorted(v) for k, v in cm.items()}}
json.dump(res, open(OUT, "w"), indent=1)
print(B, "->", A, "sources", len(st[A]), "valid strict", n_match, "comments exact", n_cexact)
for e, v in sorted(ev.items()):
    print(f"  {e}: {len(v)}")
    if e not in ("outputs_changed", "gained_match", "rejection_gained"):
        for x in v: print("     ", x, st[B][x]["status"], "->", st[A][x]["status"], st[A][x]["stderr_head"][:100].replace("\n", " "))
for e, v in cm.items():
    print(f"  comments {e}: {len(v)}")
    if e == "lost":
        for x in v: print("     ", x)
