"""Checker expectations for C11A, written before any Java run. Usage: python3 -I c11a_expect.py <batch_dir>
Per case: the group hypothesis and whether tip output differs from merge (known from rust-predictions.json)."""
import json, pathlib, sys
B = pathlib.Path(sys.argv[1]); cases = json.load(open(B / "cases.json"))["cases"]
pred = {p["name"]: p for p in json.load(open(B / "rust-predictions.json"))["predictions"]}
H = {
 "H1": "Verdana italic / bold-italic cases whose tip output differs from merge move toward Java; each such case MATCHes at tip "
       "whenever its same-consumer upright control (plain, or the creole ctl) MATCHes at tip, and is not a lost match vs merge.",
 "H2": "Verdana italic / bold-italic cases whose tip output equals merge: the consumer does not measure the italic slot; "
       "their Java status equals merge's (no gain, no loss); any failure with a matching control is a consumer gap outside the packet.",
 "H3": "Arial (all faces), other families (Times, Courier New, Helvetica, SansSerif, Monospaced, Lucida Grande): tip == merge byte-identical "
       "(already true in the predictions), so no lost match or gain.",
 "H4": "StateNodeFont italic removal (C5): every stateFontStyle form in Times / SansSerif gives tip == merge (true in predictions); "
       "Verdana sx cases change only through the italic tables.",
 "H5": "Queue pill (C3): Times queue cases tip == merge; Verdana queue cases change only in italic text widths, not in pill height.",
 "H6": "Sizes 9 and 23: per-glyph units x size reproduce Java widths for Verdana italic/bold-italic exactly where the plain control matches.",
 "H7": "Non-ASCII (na): T14.55 residue, no claim beyond 'no lost match vs merge'.",
}
rows = []
for c in cases:
    n = c["name"]; g = n.split("_")[1]; p = pred[n]
    changed = p["merge"]["stdout_sha256"] != p["tip"]["stdout_sha256"]
    styl = [a for a in c["axes"] if a in ("i", "bi", "b", "p")]
    if g == "of" or "_ar_" in n or n.endswith("_ar"): h = "H3"
    elif g == "sx": h = "H4"
    elif g == "qu": h = "H5"
    elif g == "na": h = "H7"
    elif g.startswith("sz") and changed: h = "H6"
    elif changed: h = "H1"
    else: h = "H2" if styl and styl[0] in ("i", "bi") else "unchanged"
    rows.append({"name": n, "hypothesis": h, "tip_differs_from_merge": changed})
json.dump({"batch": "C11A", "registered_before_java": True, "hypotheses": H, "cases": rows}, open(B / "checker-expectations.json", "w"), indent=1)
import collections; print(collections.Counter(r["hypothesis"] for r in rows))
