import json, sys, pathlib, collections
W = pathlib.Path(sys.argv[1])
MARK = ["Syntax Error","NoSuchElementException","Welcome to PlantUML","An error has occured","kill cannot be used here",
 "swimlane must be defined at the start","Note already created:","Parsing syntax error about %","[From string",
 "Your data does not sound like YAML data","does&#160;not&#160;sound&#160;like&#160;YAML","Your data does not sound like JSON data",
 "does&#160;not&#160;sound&#160;like&#160;JSON","No class ","(Assumed diagram type:","DITAA has crashed","This feature has been suppressed"]
st = {s: json.load(open(W/f"out-{s}/status.json")) for s in ("before","after")}
cmp = {}
for line in open(W/"sweep_cmp.tsv"):
    a, r, n, *_ = line.rstrip("\n").split("\t")
    s = "before" if "/out-before/" in a else "after"
    key = a.split(f"/out-{s}/",1)[1][:-4] + ".puml"
    cmp[(s,key)] = r
ev = collections.defaultdict(list)
for k in st["before"]:
    java = (W/"src-after/test-diagrams"/k).with_suffix(".svg").read_text(errors="replace")
    jerr = (len(java.strip()) == 0) or any(m in java for m in MARK)
    crash = "An error has occured" in java
    b, a = st["before"][k], st["after"][k]
    bs, as_ = str(b["status"]), str(a["status"])
    if bs == "0" and as_ != "0":
        ev["rejection_gained" if jerr else "new_valid_input_error"].append(k)
    if bs != "0" and as_ == "0":
        ev["lost_rejection" if jerr else "error_now_renders_valid_ref"].append(k)
    if a["panic"] and not b["panic"]: ev["new_panic"].append(k)
    if b["panic"] and not a["panic"]: ev["panic_removed"].append(k)
    if as_ == "timeout" and bs != "timeout": ev["new_timeout"].append(k)
    if not jerr:
        mb, ma = cmp[("before",k)]=="MATCH", cmp[("after",k)]=="MATCH"
        if mb and not ma: ev["lost_match"].append(k)
        if ma and not mb: ev["gained_match"].append(k)
    if b["stdout_sha256"] != a["stdout_sha256"]: ev["bytes_changed"].append(k)
json.dump(ev, open(W/"sweep_events.json","w"), indent=1, sort_keys=True)
for e, v in sorted(ev.items()):
    print(e, len(v))
    if e in ("new_valid_input_error","lost_rejection","new_panic","new_timeout","lost_match","error_now_renders_valid_ref","panic_removed"):
        for x in v: print("   ", x, st["before"][x]["status"], "->", st["after"][x]["status"])
