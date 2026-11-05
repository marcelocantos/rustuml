"""Register fresh Java pairs and compute per-pair results (after/before/comment list)."""
import base64, hashlib, json, pathlib, re, shutil, subprocess, sys
W = pathlib.Path("/Users/marcelo/Library/Caches/rustuml-shared-svg-claude-checker")
REPO = pathlib.Path(sys.argv[1])
AR = pathlib.Path("/Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-claude-batch1")
C1 = pathlib.Path("/Users/marcelo/Library/Caches/rustuml-relay-parent/shared-svg-checker-c1")
C = re.compile(r"<!--(.*?)-->", re.S)

def load(batch_dir, batch):
    res = json.load(open(batch_dir / "reference-results.json"))["results"]
    man = {c["name"]: c for c in json.load(open(batch_dir / "reference-manifest.json"))["cases"]}
    out = {}
    for r in res:
        src = pathlib.Path(r["source"]).read_bytes()
        assert hashlib.sha256(src).hexdigest() == r["source_sha256"]
        svg = base64.b64decode(r["stdout_base64"])
        assert hashlib.sha256(svg).hexdigest() == r["stdout_sha256"]
        out[r["name"]] = dict(batch=batch, src=src, svg=svg, exit=r["exit_code"],
                              stderr=base64.b64decode(r["stderr_base64"]).decode("utf-8", "replace"),
                              axes=man[r["name"]]["axes"])
    return out

refs = {**load(AR, "AR"), **load(C1, "AU (checker batch C1)")}
PLAN = {
 "tim-evaluated-event-stream": ["blank0_activity_shadow", "blank0_class_shadow", "blank1_activity_shadow", "blank1_class_shadow",
   "blank3_activity_shadow", "blank3_class_shadow", "raw_unit_test_input", "raw_blank_before_start", "raw_trailing_blanks",
   "raw_crlf_one_blank", "raw_multi_blank_first_block", "raw_include_leading_blanks", "raw_whitespace_only_lines",
   "raw_sequence_two_blanks", "theme_plain_sequence", "theme_plain_class_note", "theme_mono_class"],
 "sequence-delay-label-allocation": ["delay_legacy_leading", "delay_teoz_leading", "delay_teoz_medium", "delay_legacy_lead1",
   "delay_legacy_plain0", "delay_legacy_empty", "delay_teoz_empty", "delay_teoz_lead3",
   "delay_legacy_wide", "delay_legacy_lead2_creole", "delay_legacy_multiline", "delay_teoz_lead3_multiline"],
 "comment-state-consumers": ["cluster_entry_point", "quark_reuse_crossing", "history_depth1_autarkic", "history_depth2_crossing",
   "history_root_marker", "cluster_root_note_on_inner", "cluster_inputpin_mixed_case", "cluster_entry_inside_autarkic",
   "history_external_suffix_form", "cluster_concurrent_crossing", "deploy_alias_reverse_up", "deploy_unaliased_forms",
   "deploy_alias_direction_matrix", "deploy_nested_alias_reverse", "stroke_skinparam_css4", "stroke_state_style_transparent",
   "stroke_stereo_style_css4", "stroke_stereo_skinparam_css4", "stroke_per_state_css4", "stroke_per_arrow_css4",
   "stroke_arrow_skinparam_transparent", "stroke_zero_alpha_nonblack", "delay_legacy_short"],
}
GOLDEN_COPIES = {"sequence-delay-label-allocation": ["edge-cases/edge_seq_delay_texts", "edge-cases/edge_sequence_delay"]}
SUB = "checker-claude-shared-svg"
ERRMARK = ["Syntax Error", "Welcome to PlantUML", "An error has occured", "[From string", "(Assumed diagram type:"]
DEBUG = "date=1774210426000,tz=AEDT+1100"

def run(binary, src, cwd):
    p = subprocess.run([binary, "-pipe", "-tsvg"], input=src, capture_output=True, cwd=cwd, timeout=60,
                       env={"PATH": "/usr/bin:/bin", "RUSTUML_DEBUG": DEBUG, "TZ": "GMT+11:00"})
    return p.returncode, p.stdout

def cmp(java_path, rust_bytes, tag):
    t = W / "pairs-tmp"; t.mkdir(exist_ok=True)
    a = t / f"{tag}.svg"; a.write_bytes(rust_bytes)
    lst = t / "l.txt"; lst.write_text(f"{java_path}\t{a}\n")
    o = subprocess.run([str(W / "tgt-cmp/release/checker-cmp"), str(lst)], capture_output=True, text=True).stdout.split("\t")
    return o[1], int(o[2])

table = {}
for mech, names in PLAN.items():
    d = REPO / "test-diagrams/perturbations" / mech / SUB
    d.mkdir(parents=True, exist_ok=True)
    entries = []
    items = [(n, refs[n]["src"], refs[n]["svg"], refs[n]) for n in names]
    for g in GOLDEN_COPIES.get(mech, []):
        gp = REPO / "test-diagrams/golden" / g
        items.append((pathlib.Path(g).name, gp.with_suffix(".puml").read_bytes(), gp.with_suffix(".svg").read_bytes(),
                      dict(batch="golden copy (GET provenance) of test-diagrams/golden/" + g, exit=0, stderr="",
                           axes=["delay label without leading spacer", "fixed golden copied byte-identically for T14.6.7"])))
    if mech == "tim-evaluated-event-stream":
        shutil.copyfile(W / "batch-c1/resources/inc_leading_blank.iuml", d / "inc_leading_blank.iuml")
    for n, src, svg, r in items:
        (d / f"{n}.puml").write_bytes(src); (d / f"{n}.svg").write_bytes(svg)
        java_text = svg.decode("utf-8", "replace")
        java_error = r["exit"] != 0 or any(m in java_text for m in ERRMARK) or not svg.strip()
        row = {"source": f"test-diagrams/perturbations/{mech}/{SUB}/{n}.puml",
               "golden": f"test-diagrams/perturbations/{mech}/{SUB}/{n}.svg",
               "source_sha256": hashlib.sha256(src).hexdigest(), "golden_sha256": hashlib.sha256(svg).hexdigest(),
               "axes": r["axes"], "reference_batch": r["batch"], "java_exit": r["exit"],
               "java_stderr": r["stderr"].strip()[:200], "reference_invalid": java_error,
               "cwd": f"test-diagrams/perturbations/{mech}/{SUB}"}
        for tag, b in (("before", "tgt-before"), ("merged_predraft", "tgt-merged"), ("after", "tgt-after")):
            if tag == "merged_predraft" and mech != "sequence-delay-label-allocation":
                continue
            code, out = run(str(W / b / "release/rustuml"), src, d)
            res, nd = cmp(d / f"{n}.svg", out, f"{mech}-{n}-{tag}")
            row[f"{tag}_cli_exit"] = code
            row[f"{tag}_strict_compare"] = res if res != "MATCH" else "match"
            row[f"{tag}_difference_count"] = nd
            row[f"{tag}_comment_list_exact"] = C.findall(out.decode("utf-8", "replace")) == C.findall(java_text)
        row["result"] = "pass" if (row["after_strict_compare"] == "match" and not java_error) else "fail"
        entries.append(row)
    table[mech] = entries
(W / "pairs.json").write_text(json.dumps(table, indent=1))
for mech, rows in table.items():
    print(mech, len(rows), "pass", sum(r["result"] == "pass" for r in rows))
    for r in rows:
        print("  ", r["source"].split("/")[-1], r["result"], "before", r["before_strict_compare"], "after", r["after_strict_compare"],
              "cmt", r["before_comment_list_exact"], r["after_comment_list_exact"], "java_exit", r["java_exit"],
              ("merged " + r["merged_predraft_strict_compare"]) if "merged_predraft_strict_compare" in r else "")
