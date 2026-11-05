"""Summarize batch-final-<X>.json: matches, losses, new errors, panics, crash-page outcomes. Usage: batch_summary.py <work> <out.json> <label>..."""
import json, pathlib, re, sys, base64
W = pathlib.Path(sys.argv[1]); OUT = sys.argv[2]; labels = sys.argv[3:]
summary = {}
for L in labels:
    rows = json.load(open(W / f"batch-final-{L}.json"))
    s = {"cases": len(rows), "java_valid": 0, "java_error_pages": 0}
    for tag in ("base", "r3", "tip"):
        s[f"{tag}_strict_matches_valid"] = 0
    ev = {k: [] for k in ("lost_match_vs_base", "lost_match_vs_r3", "new_valid_error_vs_base", "new_valid_error_vs_r3",
                          "lost_rejection_vs_base", "lost_rejection_vs_r3", "new_panic_vs_base", "new_panic_vs_r3", "gained_vs_r3")}
    crash = []
    for n, r in rows.items():
        jerr = r["java_error_marker"] or r["java_exit"] != 0
        if jerr:
            s["java_error_pages"] += 1
        else:
            s["java_valid"] += 1
        for tag in ("base", "r3", "tip"):
            if not jerr and r[f"{tag}_strict"] == "MATCH":
                s[f"{tag}_strict_matches_valid"] += 1
        for before in ("base", "r3"):
            b, t = r[f"{before}_strict"] == "MATCH", r["tip_strict"] == "MATCH"
            if not jerr and b and not t: ev[f"lost_match_vs_{before}"].append(n)
            if not jerr and r[f"{before}_exit"] == 0 and r["tip_exit"] != 0: ev[f"new_valid_error_vs_{before}"].append(n)
            if jerr and r[f"{before}_exit"] != 0 and r["tip_exit"] == 0: ev[f"lost_rejection_vs_{before}"].append(n)
            if r["tip_panic"] and not r[f"{before}_panic"]: ev[f"new_panic_vs_{before}"].append(n)
        if not jerr and r["tip_strict"] == "MATCH" and r["r3_strict"] != "MATCH": ev["gained_vs_r3"].append(n)
        if r["java_crash"]:
            jt = (W / f"out-final-{L}" / f"{n}.java.svg").read_text(errors="replace")
            m = re.search(r"An error has occured : ([^<]{0,240})", jt)
            jmsg = m.group(1).replace("&#160;", " ") if m else ""
            tmsg = r["tip_stderr"].removeprefix("export error: ")
            crash.append({"name": n, "tip_exit": r["tip_exit"], "tip_stderr": r["tip_stderr"][:160],
                          "java_message_exact": bool(tmsg) and r["tip_exit"] != 0 and tmsg in jmsg})
    s.update(ev); s["java_crash_pages"] = crash
    s["comment_lists_exact"] = {tag: sum(r[f"{tag}_comments_exact"] for r in rows.values()) for tag in ("base", "r3", "tip")}
    summary[L] = s
    print(L, {k: (len(v) if isinstance(v, list) else v) for k, v in s.items() if k != "java_crash_pages"})
    if crash:
        print("   crash pages: tip exit!=0 with Java message", sum(c["java_message_exact"] for c in crash), "/", len(crash),
              [c["name"] for c in crash if not c["java_message_exact"]])
json.dump(summary, open(OUT, "w"), indent=1, ensure_ascii=False)
