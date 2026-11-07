"""Per cohort: Java validity from raw SVG + stderr, and merge-vs-reference-label equality (bytes, exit, strict).
usage: compare_cohorts.py <relay-root> <cohort-root> <out.json> cohort=reflabel ..."""
import base64, json, os, re, sys
relay, croot, outp, specs = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]
CRASH = re.compile(rb"An error has occured|An error has occurred|java\.lang\.|Exception|<title>Error</title>|PlantUML diagram syntax error|Syntax Error", re.I)
def num(s): return 0 if s == "MATCH" else (int(s.split()[1]) if s.startswith("DIFF") else None)
summary = {}
for spec in specs:
    c, ref = spec.split("=")
    r = json.load(open(os.path.join(relay, c, "reference-results.json"))); rows = r["results"] if isinstance(r, dict) else r
    sc = json.load(open(os.path.join(croot, c, "score.json")))
    rend = json.load(open(os.path.join(croot, c, "out", "render.json")))
    valid, invalid = [], {}
    for row in rows:
        n = row["name"]; so = base64.b64decode(row["stdout_base64"]); se = base64.b64decode(row.get("stderr_base64") or "")
        if row["exit_code"] != 0: invalid[n] = f"exit {row['exit_code']}"
        elif not so.lstrip().startswith((b"<svg", b"<?xml")): invalid[n] = "exit 0 non-SVG stdout"
        elif CRASH.search(so) or se.strip(): invalid[n] = "exit 0 crash/error page or stderr"
        else: valid.append(n)
    def cnt(l): return sum(sc[n][l]["strict"] == "MATCH" for n in valid)
    byte_diff = [n for n in rend if rend[n]["merge"]["stdout_sha256"] != rend[n][ref]["stdout_sha256"] or rend[n]["merge"]["exit"] != rend[n][ref]["exit"] or rend[n]["merge"]["stderr_sha256"] != rend[n][ref]["stderr_sha256"]]
    lost = [n for n in valid if sc[n][ref]["strict"] == "MATCH" and sc[n]["merge"]["strict"] != "MATCH"]
    worse = [n for n in valid if num(sc[n][ref]["strict"]) is not None and num(sc[n]["merge"]["strict"]) is not None and num(sc[n]["merge"]["strict"]) > num(sc[n][ref]["strict"])]
    summary[c] = {"reference_label": ref, "cases": len(rows), "java_valid": len(valid), "java_invalid": invalid,
                  "strict_match": {l: cnt(l) for l in ("base", "g4", "merge")},
                  "merge_vs_ref_byte_or_exit_or_stderr_differs": byte_diff, "merge_lost_vs_ref": lost, "merge_worse_vs_ref": worse,
                  "panics": {l: sum(rend[n][l]["exit"] == 101 for n in rend) for l in ("base", "g4", "merge")}}
    s = summary[c]
    print(f"{c:26} ref={ref:4} valid {len(valid)}/{len(rows)} strict base {s['strict_match']['base']} g4 {s['strict_match']['g4']} merge {s['strict_match']['merge']} | merge!=ref bytes {len(byte_diff)} lost {len(lost)} worse {len(worse)} panics {s['panics']}")
json.dump(summary, open(outp, "w"), indent=1, sort_keys=True)
