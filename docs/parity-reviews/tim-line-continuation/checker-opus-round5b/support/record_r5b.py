"""Write the round-5 recheck record tables: per-case DJ/DH, worse lists with attribution, and the +1 line-shift test.
Usage: python3 -I record_r5b.py RECORD_DIR"""
import hashlib, json, re, subprocess, sys
from pathlib import Path
V = Path.home() / "Library/Caches/rustuml-opus-tim-r5-recheck"; D = Path(sys.argv[1])
sys.path.insert(0, str(V / "support")); from classify import outcome
B = ["6fd071169", "1ad869dae", "9ed52f435", "28af44ab3"]
s = json.loads((V / "summary-ov.json").read_text()); t = json.loads((V / "table-ov.json").read_text())
appr = {r["case"] for r in json.loads(Path("/tmp/r5b_approved.json").read_text())["rows"]}
def rows(co, b): return {r["name"] if "name" in r else r["source"]: r for r in json.loads((V / f"runs/{co}-{b}/results.json").read_text())["rows"]}
shift = {}
for co in ["r5b", "dh"]:
    man = {m["name"]: m for m in json.loads((V / f"fresh-{co}-manifest.json").read_text())}
    R = {b: rows(co, b) for b in B}; out = []
    for n, m in man.items():
        src = (V / f"fresh-{co}/{n}.puml").read_bytes().decode(errors="replace")
        mark = any(l.startswith("@start") for l in src.replace("\r", "").split("\n"))
        r = dict(name=n, java_exit=m["java_exit"], prepare_observed_valid=m["observed_valid"],
                 overlay_valid=m["observed_valid"] and str(m["java_exit"]) not in ("100", "50") and b"not supported by this release" not in (V / f"fresh-{co}/{m['golden']}").read_bytes(),
                 pipe_column0_mark=mark, outcome={b: outcome(R[b][n]) for b in B},
                 tip_stderr=R[B[0]][n]["results"]["svg"]["stderr"].strip()[:200])
        if r["outcome"][B[0]] == "mismatch" and r["overlay_valid"]:
            key = hashlib.sha256(f"{n}.puml".encode()).hexdigest()[:24]
            svg = (V / f"runs/{co}-{B[0]}/{key}.svg").read_text()
            sh = re.sub(r'(data-source-line|codeLine)="(\d+)"', lambda x: f'{x.group(1)}="{int(x.group(2)) + 1}"', svg)
            p = V / "scratch/shift.svg"; p.write_text(sh)
            r["tip_plus1_line_shift_compare_exit"] = subprocess.run([str(V / "bin/compare_cli"), str(V / f"fresh-{co}/{m['golden']}"), str(p)], capture_output=True).returncode
            shift[f"{co}:{n}"] = dict(pipe_column0_mark=mark, compare_exit=r["tip_plus1_line_shift_compare_exit"])
        out.append(r)
    (D / f"results/{co}-percase.json").write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
(D / "results/plus1-line-shift.json").write_text(json.dumps(dict(note="Tip SVG with every data-source-line and codeLine incremented by 1 (Pipe.java:217 prepends @startuml when no line startsWith('@start')), compared with Java by the unchanged compare_cli. 0 = match.", cases=shift), indent=1) + "\n")
NUMBERING = {"r5b_num_bs_seq_three", "r5b_num_upper_at_seq", "r5b_num_us_bs_seq", "r5b_num_bs_blank_inside", "r5b_start_bs_name", "r5b_end_bs_bare",
             "r5b_multi_bs_then_bs_mindmap", "r5b_crlf_bs_seq", "r5b_num_bs_state", "r5b_num_bs_object", "r5b_num_bs_deployment", "o5s_bs_uml_class",
             "o5s_mix_bs_start_at_end", "o5s_ws_tab_bs", "o5s_ws_bs_end_indented", "o5s_case_bs_upper_start", "o5s_case_at_upper_start_lower_end"}
TYPE_WORD = {"r5b_start_bs_chen", "r5b_start_bs_project", "r5b_start_bs_space_uml"}
NBSP = {"r5b_start_nbsp"}
def attribution(co, n, b):
    if n in appr: return "approved (review-5 merge/worse-vs-32684f81e.json)"
    if n in NUMBERING: return "PACKET: ea5375293 numbers only marker-carrying lines from Pipe's prepended @startuml; plain lines keep n-1"
    if n in TYPE_WORD: return "PACKET: b24935a2d detect_type takes the first whitespace word after start, not DiagramType#getTypes at the next character"
    if n in NBSP: return "PACKET: b24935a2d start test (java_is_whitespace) makes the NBSP line content; Java renders it, base rendered it"
    return "inherited: tip outcome equals 1ad869dae and 9ed52f435 (sprite/participant and earlier TIM rounds); 28af44ab3 differs"
for b in ["1ad869dae", "9ed52f435", "28af44ab3"]:
    lst = []
    for co, r in t.items():
        if b not in r: continue
        for k, v in r[b]["worse_by_kind"].items():
            for n in v: lst.append(dict(cohort=co, case=n, kind=k, in_review5_approved=n in appr, attribution=attribution(co, n, b)))
    (D / f"merge/worse-vs-{b}.json").write_text(json.dumps(dict(note=f"Every case where tip 6fd071169 is worse than {b} (T14.63 overlay plus the declared validity overlay).", total=len(lst),
        outside_approved=sum(not x["in_review5_approved"] for x in lst), rows=lst), indent=1) + "\n")
    print(b, len(lst), "outside approved", sum(not x["in_review5_approved"] for x in lst))
