"""Summarize batch-<label>.json files: strict/comments per tree, lost/gained, errors, panics, stderr changes.
Usage: python3 -I batchsum.py <work> <out.json> <label>..."""
import json, pathlib, sys
W = pathlib.Path(sys.argv[1]); out = {}
for L in sys.argv[3:]:
    rows = json.load(open(W / f"batch-{L}.json")); s = {"cases": len(rows), "java_exit_0": sum(r["java_exit"] == 0 for r in rows.values())}
    valid = {n for n, r in rows.items() if r["java_exit"] == 0 and not r["java_error_marker"] and not r["java_crash"]}
    s["java_valid"] = len(valid)
    for t in ["base", "merge", "tip"]:
        s[f"{t}_strict"] = sum(r[f"{t}_strict"] == "MATCH" for r in rows.values())
        s[f"{t}_strict_and_comments"] = sum(r[f"{t}_strict"] == "MATCH" and r.get(f"{t}_comments_exact", True) for r in rows.values())
        s[f"{t}_panics"] = sorted(n for n, r in rows.items() if r[f"{t}_panic"])
    for b in ["base", "merge"]:
        s[f"lost_vs_{b}"] = sorted(n for n, r in rows.items() if r[f"{b}_strict"] == "MATCH" and r["tip_strict"] != "MATCH")
        s[f"gained_vs_{b}"] = sorted(n for n, r in rows.items() if r[f"{b}_strict"] != "MATCH" and r["tip_strict"] == "MATCH")
        s[f"comments_lost_vs_{b}"] = sorted(n for n, r in rows.items() if r.get(f"{b}_comments_exact") and not r.get("tip_comments_exact"))
        s[f"new_valid_input_error_vs_{b}"] = sorted(n for n in valid if rows[n][f"{b}_exit"] == 0 and rows[n]["tip_exit"] != 0)
        s[f"lost_rejection_vs_{b}"] = sorted(n for n, r in rows.items() if n not in valid and r[f"{b}_exit"] != 0 and r["tip_exit"] == 0)
        s[f"stderr_changed_vs_{b}"] = sorted(n for n, r in rows.items() if r[f"{b}_stderr_sha256"] != r["tip_stderr_sha256"])
        s[f"new_panic_vs_{b}"] = sorted(n for n, r in rows.items() if r["tip_panic"] and not r[f"{b}_panic"])
        s[f"output_changed_vs_{b}"] = sum(r[f"{b}_sha256"] != r["tip_sha256"] for r in rows.values())
    s["tip_failures"] = {n: r["tip_ndiff"] for n, r in rows.items() if r["tip_strict"] != "MATCH"}
    out[L] = s
json.dump(out, open(sys.argv[2], "w"), indent=1)
for L, s in out.items():
    print(L, {k: (v if not isinstance(v, (list, dict)) else (v if len(v) <= 8 else len(v))) for k, v in s.items() if k not in ("tip_failures",) and not k.startswith("gained") and not k.startswith("output")})
