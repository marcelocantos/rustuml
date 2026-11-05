"""Summarize batch-<L>.json files across trees. Usage: batch_summary.py <work> <out.json> <label>..."""
import json, sys, pathlib
W = pathlib.Path(sys.argv[1]); OUT = sys.argv[2]; LABELS = sys.argv[3:]
TAGS = ["base", "r3", "tip"]; out = {}
for L in LABELS:
    rows = json.load(open(W / f"batch-{L}.json"))
    valid = {n: not (r["java_crash"] or r["java_error_marker"] or str(r["java_exit"]) != "0") for n, r in rows.items()}
    s = {"cases": len(rows), "java_valid": sum(valid.values()), "java_error_or_unavailable": [n for n, v in valid.items() if not v]}
    for t in TAGS:
        s[f"{t}_valid_match"] = sum(r[f"{t}_strict"] == "MATCH" for n, r in rows.items() if valid[n])
        s[f"{t}_comments_exact_valid"] = sum(bool(r.get(f"{t}_comments_exact")) for n, r in rows.items() if valid[n])
        s[f"{t}_panics"] = [n for n, r in rows.items() if r[f"{t}_panic"]]
        s[f"{t}_nonzero_exit"] = [n for n, r in rows.items() if r[f"{t}_exit"] != 0]
    for b in ("base", "r3"):
        ev = {}
        for n, r in rows.items():
            mb, mt = r[f"{b}_strict"] == "MATCH", r["tip_strict"] == "MATCH"
            if valid[n]:
                if mb and not mt: ev.setdefault("lost_match", []).append(n)
                if mt and not mb: ev.setdefault("gained_match", []).append(n)
                if r[f"{b}_exit"] == 0 and r["tip_exit"] != 0: ev.setdefault("new_valid_input_error", []).append(n)
                if r.get(f"{b}_comments_exact") and not r.get("tip_comments_exact") and r["format"] == "-tsvg": ev.setdefault("comments_lost", []).append(n)
            else:
                if r[f"{b}_exit"] != 0 and r["tip_exit"] == 0: ev.setdefault("lost_rejection", []).append(n)
                if r[f"{b}_exit"] == 0 and r["tip_exit"] != 0: ev.setdefault("rejection_gained", []).append(n)
            if r["tip_panic"] and not r[f"{b}_panic"]: ev.setdefault("new_panic", []).append(n)
            if r[f"{b}_sha256"] != r["tip_sha256"]: ev.setdefault("outputs_changed", []).append(n)
            if r[f"{b}_stderr_sha256"] != r["tip_stderr_sha256"]: ev.setdefault("stderr_changed", []).append(n)
        s[f"{b}_to_tip"] = ev
    s["tip_failures_valid"] = {n: {"format": r["format"], "strict": r["tip_strict"], "ndiff": r["tip_ndiff"], "exit": r["tip_exit"], "diff_head": r.get("tip_diff_head", "")[:300]} for n, r in rows.items() if valid[n] and r["tip_strict"] != "MATCH"}
    s["java_error_cases"] = {n: {"java_exit": r["java_exit"], "java_crash": r["java_crash"], "tip_exit": r["tip_exit"], "tip_stderr": r["tip_stderr"][:200], "java_stderr_head": r["java_stderr"][:200]} for n, r in rows.items() if not valid[n]}
    out[L] = s
json.dump(out, open(OUT, "w"), indent=1, ensure_ascii=False)
for L, s in out.items():
    print("==", L, {k: v for k, v in s.items() if k.endswith("_match") or k in ("cases", "java_valid")})
    for b in ("base", "r3"): print("  ", b, "->tip", {k: (v if k not in ("outputs_changed", "gained_match", "stderr_changed") else len(v)) for k, v in s[f"{b}_to_tip"].items()})
    print("   tip valid failures:", {n: (x["format"], x["strict"], x["ndiff"]) for n, x in s["tip_failures_valid"].items()})
