"""Summarise relay batches: per tree strict matches, losses/gains, lost rejections, new valid-input errors, panics.
Usage: python3 -I batchsum10.py <work> <labels...>"""
import json, sys, pathlib
W = pathlib.Path(sys.argv[1]); out = {}
for L in sys.argv[2:]:
    rows = json.load(open(W / f"batch-{L}.json"))
    M = lambda r, t: r[t + "_strict"] == "MATCH" and r.get(t + "_comments_exact", True)
    s = {"n": len(rows)}
    for t in ("base", "merge", "tip"): s[t] = sum(M(r, t) for r in rows.values())
    s["lost_merge_to_tip"] = [n for n, r in rows.items() if M(r, "merge") and not M(r, "tip")]
    s["lost_base_to_tip"] = [n for n, r in rows.items() if M(r, "base") and not M(r, "tip")]
    s["gained_merge_to_tip"] = [n for n, r in rows.items() if M(r, "tip") and not M(r, "merge")]
    jerr = lambda r: r["java_exit"] != 0 or r["java_error_marker"]
    s["lost_rejections"] = [n for n, r in rows.items() if jerr(r) and r["merge_exit"] != 0 and r["tip_exit"] == 0]
    s["new_valid_input_errors"] = [n for n, r in rows.items() if not jerr(r) and r["merge_exit"] == 0 and r["tip_exit"] != 0]
    s["panics_tip"] = [n for n, r in rows.items() if r["tip_panic"]]
    s["stderr_changed_merge_to_tip"] = [n for n, r in rows.items() if r["merge_stderr_sha256"] != r["tip_stderr_sha256"]]
    s["tip_output_changed_vs_merge"] = sum(r["merge_sha256"] != r["tip_sha256"] for r in rows.values())
    out[L] = s
json.dump(out, open(W / "batch-summary10.json", "w"), indent=1)
for L, s in out.items():
    print(L, s["n"], "base", s["base"], "merge", s["merge"], "tip", s["tip"], "lost", len(s["lost_merge_to_tip"]), len(s["lost_base_to_tip"]),
          "gain", len(s["gained_merge_to_tip"]), "lostrej", len(s["lost_rejections"]), "newerr", len(s["new_valid_input_errors"]),
          "panic", len(s["panics_tip"]), "stderrΔ", len(s["stderr_changed_merge_to_tip"]), "outΔ", s["tip_output_changed_vs_merge"])
