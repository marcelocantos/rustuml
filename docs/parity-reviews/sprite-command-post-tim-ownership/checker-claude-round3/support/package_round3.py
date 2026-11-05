"""Register round-3 checker pairs and write evidence into each mechanism's own directories (T14.6.7).

Usage: python3 -I package_round3.py REPO
Pairs: Java-valid (raw SVG + stderr markers) AV and AX cases, copied byte-identically from the hash-checked
replay dirs. Resource-dependent cases (!include/!includesub/!theme of a sibling file) stay in evidence only:
the perturbation harness parses with base None and cannot resolve them.
Sprite mechanism (rejected): every Java-valid pair, result pass iff CLI whole-output match after.
Participant mechanism (accepted): only pairs that match after; the others are listed in evidence.
"""
import hashlib, json, os, shutil, sys
from pathlib import Path

C = Path(os.path.expanduser("~/Library/Caches/rustuml-sprite-round3-claude-checker"))
sys.path.insert(0, str(C / "support"))
from classify import outcome  # noqa: E402

REPO = Path(sys.argv[1])
SP, PA = "sprite-command-post-tim-ownership", "sequence-participant-silent-residue"
RESOURCE_DEPENDENT = {"ck3_includesub_resource_one_sub_seq", "ck3_include_whole_with_subs_seq", "ck3_include_then_includesub_seq",
                      "ck3_includesub_pixel_sprite_seq", "ck3_theme_local_startsub_seq"}
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
pairs = {SP: [], PA: []}
evidence_only = {SP: [], PA: []}
for cohort in ["av", "ax"]:
    manifest = json.loads((C / f"fresh-{cohort}-manifest.json").read_text())
    after = {r["source"]: r for r in json.loads((C / f"runs/{cohort}-607890d0d/results.json").read_text())["rows"]}
    for case in manifest:
        mech = case["mechanism"]
        row = after[case["source"]]
        result = outcome(row)
        entry = dict(name=case["name"], cohort=cohort.upper(), outcome_after=result, axes=case["axes"])
        if not case["observed_valid"] or case["name"] in RESOURCE_DEPENDENT or (mech == PA and result != "match"):
            evidence_only[mech].append(dict(entry, java_valid=case["observed_valid"],
                reason="Java-invalid reference" if not case["observed_valid"] else
                       "needs a sibling resource file" if case["name"] in RESOURCE_DEPENDENT else "mismatch in every build (not caused by the packet)"))
            continue
        dest = REPO / "test-diagrams/perturbations" / mech
        for ext in ("puml", "svg"):
            src = C / f"fresh-{cohort}" / f"{case['name']}.{ext}"
            target = dest / f"{case['name']}.{ext}"
            assert not target.exists(), target
            shutil.copyfile(src, target)
            assert sha(src) == sha(target)
        pairs[mech].append(dict(source=f"test-diagrams/perturbations/{mech}/{case['name']}.puml",
                                golden=f"test-diagrams/perturbations/{mech}/{case['name']}.svg",
                                result="pass" if result == "match" else "fail", axes=case["axes"] + [f"{cohort.upper()} held out", f"after: {result}"]))
for mech in pairs:
    out = REPO / "docs/parity-reviews" / mech / "checker-claude-round3"
    out.mkdir(parents=True, exist_ok=True)
    (out / "registered-pairs.json").write_text(json.dumps(pairs[mech], indent=1) + "\n")
    (out / "evidence-only-cases.json").write_text(json.dumps(evidence_only[mech], indent=1) + "\n")
    print(mech, "pairs", len(pairs[mech]), "pass", sum(p["result"] == "pass" for p in pairs[mech]), "evidence-only", len(evidence_only[mech]))
