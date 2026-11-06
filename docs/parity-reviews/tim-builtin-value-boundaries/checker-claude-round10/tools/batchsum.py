"""Summarize batch-<L>.json: per tree strict/comments, lost/gained base->tip and merge->tip, outputs changed, exits, panics.
Usage: python3 -I batchsum.py <work> <out.json> <label>..."""
import json, sys, pathlib
W = pathlib.Path(sys.argv[1]); out = {}
for L in sys.argv[3:]:
    rows = json.load(open(W / f"batch-{L}.json")); s = {"cases": len(rows)}
    valid = {n for n, r in rows.items() if not r["java_error_marker"] and r["java_exit"] == 0}
    s["java_valid"] = len(valid); s["java_exit_0"] = sum(r["java_exit"] == 0 for r in rows.values())
    ok = lambda r, t: r[f"{t}_strict"] == "MATCH" and r.get(f"{t}_comments_exact", True)
    for t in ["base", "merge", "tip"]:
        s[f"{t}_strict"] = sum(r[f"{t}_strict"] == "MATCH" for r in rows.values())
        s[f"{t}_strict_and_comments"] = sum(ok(r, t) for r in rows.values())
        s[f"{t}_panics"] = [n for n, r in rows.items() if r[f"{t}_panic"]]
        s[f"{t}_nonzero_exit"] = [n for n, r in rows.items() if r[f"{t}_exit"] != 0]
    for b in ["base", "merge"]:
        s[f"lost_{b}_to_tip"] = [n for n, r in rows.items() if ok(r, b) and not ok(r, "tip")]
        s[f"gained_{b}_to_tip"] = [n for n, r in rows.items() if ok(r, "tip") and not ok(r, b)]
        s[f"outputs_changed_{b}_to_tip"] = [n for n, r in rows.items() if r[f"{b}_sha256"] != r["tip_sha256"]]
        s[f"stderr_changed_{b}_to_tip"] = [n for n, r in rows.items() if r[f"{b}_stderr_sha256"] != r["tip_stderr_sha256"]]
        s[f"new_valid_input_error_{b}_to_tip"] = [n for n, r in rows.items() if n in valid and r[f"{b}_exit"] == 0 and r["tip_exit"] != 0]
        s[f"lost_rejection_{b}_to_tip"] = [n for n, r in rows.items() if n not in valid and r[f"{b}_exit"] != 0 and r["tip_exit"] == 0]
        s[f"comments_lost_{b}_to_tip"] = [n for n, r in rows.items() if r.get(f"{b}_comments_exact") and not r.get("tip_comments_exact")]
    s["tip_failing_valid"] = sorted(n for n in valid if not ok(rows[n], "tip"))
    out[L] = s
json.dump(out, open(sys.argv[2], "w"), indent=1)
for L, s in out.items():
    print(L, {k: (v if not isinstance(v, list) else (len(v) if len(v) > 12 else v)) for k, v in s.items() if not k.startswith("tip_failing")})
