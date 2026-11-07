"""Retained/golden summary vs maker claims and the approved exception set (by source hash)."""
import collections, json, sys
sys.path.insert(0, sys.argv[1] + "/support")
from classify import outcome, load
R, label = sys.argv[1], sys.argv[2]
b, a = load(f"{R}/runs/{label}-before/results.json"), load(f"{R}/runs/{label}-after/results.json")
appr = json.load(open(sys.argv[3]))
approved = collections.Counter(x["source_sha256"] for x in appr)
res = dict(entries=len(b), distinct_sources=len({r["source_sha256"] for r in b}))
pairs = collections.Counter((outcome(x), outcome(y)) for x, y in zip(b, a))
res["before"] = dict(collections.Counter(k[0] for k in pairs.elements()))
res["after"] = dict(collections.Counter(k[1] for k in pairs.elements()))
res["transitions"] = {f"{k[0]}->{k[1]}": v for k, v in sorted(pairs.items()) if k[0] != k[1]}
res["gains"] = [y["source"] for x, y in zip(b, a) if outcome(x) != "match" and outcome(y) == "match"]
res["lost_matches"] = [y["source"] for x, y in zip(b, a) if outcome(x) == "match" and outcome(y) != "match"]
res["lost_rejections"] = [y["source"] for x, y in zip(b, a) if outcome(x) == "nonpositive-rejected" and outcome(y) != "nonpositive-rejected"]
res["new_rejections"] = [y["source"] for x, y in zip(b, a) if outcome(x) == "nonpositive-accepted" and outcome(y) == "nonpositive-rejected"]
res["new_valid_errors"] = [y["source"] for x, y in zip(b, a) if outcome(x) != "valid-error" and outcome(y) == "valid-error"]
res["recovered_valid_errors"] = [y["source"] for x, y in zip(b, a) if outcome(x) == "valid-error" and outcome(y) != "valid-error"]
errs = [y for y in a if outcome(y) == "valid-error"]
res["after_valid_errors"] = [dict(source=y["source"], source_sha256=y["source_sha256"], approved=y["source_sha256"] in approved,
                              stderr=y["results"]["svg"]["stderr"].strip().splitlines()[-1][:200] if y["results"]["svg"]["stderr"].strip() else "") for y in errs]
res["after_valid_error_hashes_outside_approved"] = sorted({y["source_sha256"] for y in errs} - set(approved))
res["approved_entries_present_as_errors"] = sum(1 for y in errs if y["source_sha256"] in approved)
res["panics"] = [y["source"] for y in a if outcome(y) == "panic"] + [x["source"] for x in b if outcome(x) == "panic"]
res["model_mode_panics"] = {m: [y["source"] for y in side if y["results"][m]["panic"]] for side in (a,) for m in ("ast","ascii","yaml")}
res["model_mode_panics_before"] = {m: len([x for x in b if x["results"][m]["panic"]]) for m in ("ast","ascii","yaml")}
res["timeouts"] = [y["source"] for y in a if outcome(y) == "timeout"]
res["svg_byte_changes"] = sum(x["results"]["svg"]["stdout_sha256"] != y["results"]["svg"]["stdout_sha256"] for x, y in zip(b, a))
res["model_changes"] = {m: sum(x["results"][m]["stdout_sha256"] != y["results"][m]["stdout_sha256"] for x, y in zip(b, a)) for m in ("ast", "ascii", "yaml")}
json.dump(res, open(f"{R}/{label}-summary.json", "w"), indent=1)
print(json.dumps({k: (v if not isinstance(v, list) else len(v)) for k, v in res.items()}, indent=1))
