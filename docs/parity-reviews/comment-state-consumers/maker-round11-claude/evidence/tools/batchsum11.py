"""Summarise relay batches merge -> tip: matches, losses, rejections, errors, panics, stderr, exact viewBox and
comment list. Usage: python3 -I batchsum11.py <work> <out.json> <label>..."""
import json, re, sys, pathlib
W = pathlib.Path(sys.argv[1]); out = {}
VB = re.compile(rb'viewBox="([^"]*)"'); CM = re.compile(rb"<!--(.*?)-->", re.S)
for L in sys.argv[3:]:
    d = json.load(open(W / f"batch-{L}.json")); row = {"n": len(d)}
    valid = lambda r: r["java_exit"] == 0 and not r["java_error_marker"] and not r["java_crash"]
    row["merge"] = sum(r["merge_strict"] == "MATCH" for r in d.values())
    row["tip"] = sum(r["tip_strict"] == "MATCH" for r in d.values())
    row["lost"] = [n for n, r in d.items() if r["merge_strict"] == "MATCH" and r["tip_strict"] != "MATCH"]
    row["gained"] = [n for n, r in d.items() if r["tip_strict"] == "MATCH" and r["merge_strict"] != "MATCH"]
    row["lost_rejections"] = [n for n, r in d.items() if not valid(r) and r["merge_exit"] != 0 and r["tip_exit"] == 0]
    row["new_valid_input_errors"] = [n for n, r in d.items() if valid(r) and r["merge_exit"] == 0 and r["tip_exit"] != 0]
    row["panics_tip"] = [n for n, r in d.items() if r["tip_panic"]]
    row["new_panics"] = [n for n, r in d.items() if r["tip_panic"] and not r["merge_panic"]]
    row["stderr_changed"] = [n for n, r in d.items() if r["merge_stderr_sha256"] != r["tip_stderr_sha256"]]
    row["outputs_changed"] = [n for n, r in d.items() if r["merge_sha256"] != r["tip_sha256"]]
    vb = {"merge": [], "tip": []}; cm = {"merge": [], "tip": []}
    for n, r in d.items():
        if r["format"] != "-tsvg": continue
        j = (W / f"out-{L}/{n}.java.svg").read_bytes()
        for t in ("merge", "tip"):
            a = (W / f"out-{L}/{n}.{t}.svg").read_bytes()
            if VB.findall(a) == VB.findall(j): vb[t].append(n)
            if CM.findall(a) == CM.findall(j): cm[t].append(n)
    row["viewbox_exact"] = {t: len(v) for t, v in vb.items()}; row["comments_exact"] = {t: len(v) for t, v in cm.items()}
    row["viewbox_lost"] = sorted(set(vb["merge"]) - set(vb["tip"])); row["viewbox_gained"] = sorted(set(vb["tip"]) - set(vb["merge"]))
    row["comments_lost"] = sorted(set(cm["merge"]) - set(cm["tip"])); row["comments_gained"] = sorted(set(cm["tip"]) - set(cm["merge"]))
    out[L] = row
    print(f"{L}: n={row['n']} merge={row['merge']} tip={row['tip']} lost={len(row['lost'])} gained={len(row['gained'])} "
          f"lostrej={len(row['lost_rejections'])} newerr={len(row['new_valid_input_errors'])} panics_tip={len(row['panics_tip'])} "
          f"newpanic={len(row['new_panics'])} stderr_chg={len(row['stderr_changed'])} out_chg={len(row['outputs_changed'])} "
          f"vb {row['viewbox_exact']} lost={row['viewbox_lost']} gained={len(row['viewbox_gained'])} cm {row['comments_exact']} lost={row['comments_lost']}")
json.dump(out, open(sys.argv[2], "w"), indent=1)
