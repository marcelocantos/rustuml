"""Split sweep classification by cohort. Usage: cohorts.py <work> <repo> <out.json>"""
import json, sys, pathlib, re, collections
W, REPO, OUT = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3]
TREES = ["base", "r3", "d5"]
MARK = ["Syntax Error","NoSuchElementException","Welcome to PlantUML","An error has occured","kill cannot be used here",
 "swimlane must be defined at the start","Note already created:","Parsing syntax error about %","[From string",
 "Your data does not sound like YAML data","does&#160;not&#160;sound&#160;like&#160;YAML","Your data does not sound like JSON data",
 "does&#160;not&#160;sound&#160;like&#160;JSON","No class ","(Assumed diagram type:","DITAA has crashed","This feature has been suppressed"]
GET = {"test-diagrams/golden/activity/act_skinparam_shadowing.puml","test-diagrams/golden/activity/act_skinparam2_shadowing_true.puml",
       "test-diagrams/golden/edge-cases/edge_whitespace_multiple_blank_lines.puml"}
C = re.compile(r"<!--(.*?)-->", re.S)
st = {s: json.load(open(W / f"sweep-{s}/status.json")) for s in TREES}
cmp = {}
for line in open(W / "cmp.tsv"):
    a, r, *_ = line.rstrip("\n").split("\t")
    for s in TREES:
        tag = f"/sweep-{s}/"
        if tag in a: cmp[(s, a.split(tag, 1)[1][:-4] + ".puml")] = r
res = {}
for cohort, pred in [("goldens", lambda k: k.startswith("test-diagrams/golden/")),
                     ("perturbations", lambda k: k.startswith("test-diagrams/perturbations/")),
                     ("all", lambda k: True)]:
    keys = [k for k in sorted(st[TREES[-1]]) if pred(k)]
    row = {"sources": len(keys)}
    jerr = {}
    for k in keys:
        ref = (REPO / k).with_suffix(".svg").read_text(errors="replace")
        jerr[k] = (len(ref.strip()) == 0) or any(m in ref for m in MARK)
        jc = C.findall(ref)
        for s in TREES:
            o = (W / f"sweep-{s}" / k).with_suffix(".svg").read_text(errors="replace")
            row.setdefault(f"{s}_comments_exact", 0); row[f"{s}_comments_exact"] += C.findall(o) == jc
    row["java_valid"] = sum(not v for v in jerr.values()); row["java_error"] = sum(jerr.values())
    for s in TREES:
        row[f"{s}_valid_strict_match"] = sum(cmp[(s, k)] == "MATCH" for k in keys if not jerr[k])
        row[f"{s}_exit"] = dict(collections.Counter(str(st[s][k]["status"]) for k in keys))
        row[f"{s}_panics"] = sum(st[s][k]["panic"] for k in keys)
        row[f"{s}_timeouts"] = sum(st[s][k]["status"] == "timeout" for k in keys)
        row[f"{s}_java_error_rejected"] = sum(str(st[s][k]["status"]) != "0" for k in keys if jerr[k])
    for b in TREES[:-1]:
        a = TREES[-1]; ev = collections.defaultdict(list)
        for k in keys:
            mb, ma = cmp[(b, k)] == "MATCH", cmp[(a, k)] == "MATCH"
            bs, as_ = str(st[b][k]["status"]), str(st[a][k]["status"])
            if not jerr[k]:
                if mb and not ma: ev["lost_match_GET_provenance" if k in GET else "lost_match"].append(k)
                if ma and not mb: ev["gained_match"].append(k)
                if bs == "0" and as_ != "0": ev["new_valid_input_error"].append(k)
                if bs != "0" and as_ == "0": ev["error_now_renders_valid_ref"].append(k)
            else:
                if bs != "0" and as_ == "0": ev["lost_rejection"].append(k)
                if bs == "0" and as_ != "0": ev["rejection_gained"].append(k)
            if st[a][k]["panic"] and not st[b][k]["panic"]: ev["new_panic"].append(k)
            if as_ == "timeout" and bs != "timeout": ev["new_timeout"].append(k)
            if st[a][k]["stdout_sha256"] != st[b][k]["stdout_sha256"]: ev["outputs_changed"].append(k)
            jc = C.findall((REPO / k).with_suffix(".svg").read_text(errors="replace"))
            eb = C.findall((W / f"sweep-{b}" / k).with_suffix(".svg").read_text(errors="replace")) == jc
            ea = C.findall((W / f"sweep-{a}" / k).with_suffix(".svg").read_text(errors="replace")) == jc
            if eb and not ea: ev["comments_lost"].append(k)
            if ea and not eb: ev["comments_gained"].append(k)
        row[f"{b}_to_{a}"] = {e: (v if e not in ("outputs_changed", "comments_gained", "rejection_gained", "gained_match") or len(v) <= 12 else len(v)) for e, v in sorted(ev.items())}
        row[f"{b}_to_{a}_counts"] = {e: len(v) for e, v in sorted(ev.items())}
    res[cohort] = row
res["GET_goldens"] = {k: {s: {"exit": st[s][k]["status"], "strict": cmp[(s, k)]} for s in TREES} for k in sorted(GET)}
json.dump(res, open(OUT, "w"), indent=1)
for c, r in res.items():
    if c == "GET_goldens": continue
    print(c, {k: v for k, v in r.items() if not k.endswith("_to_e6f629944")})
print(json.dumps(res["GET_goldens"]))
