"""Package the checker evidence into the repo: perturbation pairs per mechanism, guard-schema review.json,
full checker-claude/review.json, run summaries and copies of shared evidence (SHA256 recorded)."""
import collections, gzip, hashlib, json, shutil, subprocess, sys
from pathlib import Path
R = Path("/Users/marcelo/Library/Caches/rustuml-sprite-ownership-claude-checker"); REPO = Path(sys.argv[1])
sys.path.insert(0, str(R / "support")); from classify import outcome
sha = lambda b: hashlib.sha256(b).hexdigest()
SP, PA = "sprite-command-post-tim-ownership", "sequence-participant-silent-residue"
rev = lambda c: subprocess.check_output(["git", "-C", str(REPO), "rev-parse", c], text=True).strip()
C = {k: rev(v) for k, v in dict(before="93bf53e18", sprite_account="ff761cd0a", sprite_impl="95cf85252", part_account="65b5a1633",
     part_impl1="ecbd75ec8", part_impl2="87bdb0feb", maker_tip="5cf1eac12", base="9834a7194", rejection="dc1dfbb4a").items()}
JAVA = "71806a23780b04a5ccde2f8ceb5121edad5eb711"

def load(p): return json.loads(Path(p).read_text())["rows"]
man = {m["source"]: m for m in json.loads((R / "fresh-manifest.json").read_text()) + json.loads((R / "fresh-at-manifest.json").read_text())}
srcdir = {**{m["source"]: R / "fresh" for m in json.loads((R / "fresh-manifest.json").read_text())},
          **{m["source"]: R / "fresh-at" for m in json.loads((R / "fresh-at-manifest.json").read_text())}}
B = load(R / "runs/fresh-before/results.json") + load(R / "runs/at-before/results.json")
A = load(R / "runs/fresh-after/results.json") + load(R / "runs/at-after/results.json")
PART_WORDS = ("redeclare", "multi_stereo", "tooltip", "note_across", "hnote", "rnote", "box_plain", "group_space")
def mech_of(m):
    if m["cohort"] == "AQ": return SP
    if m["cohort"] in ("AS", "AT"): return m["mechanism"]
    return None  # 790/28: already preserved under their own (earlier) mechanism directories
rows = collections.defaultdict(list)
for b, a in zip(B, A):
    assert b["source"] == a["source"]; m = man[b["source"]]; mech = mech_of(m)
    ob, oa = outcome(b), outcome(a)
    rows[mech].append(dict(name=m["name"], cohort=m["cohort"], java_valid=m["observed_valid"], intended_valid=m["intended_valid"],
        java_markers=m["java_markers"], axes=m["axes"], before=ob, after=oa,
        before_svg=b["results"]["svg"]["stdout_sha256"], after_svg=a["results"]["svg"]["stdout_sha256"],
        after_stderr=a["results"]["svg"]["stderr"][-600:], java_svg_sha256=m["java_svg_sha256"], source_sha256=b["source_sha256"]))
def classify_change(r):
    if r["java_valid"]:
        if r["before"] == "match" and r["after"] != "match": return "lost-match"
        if r["after"] == "valid-error" and r["before"] != "valid-error": return "new-valid-error"
        if r["after"] == "match" and r["before"] != "match": return "gain"
    else:
        if r["before"] == "nonpositive-rejected" and r["after"] != "nonpositive-rejected": return "lost-rejection"
        if r["after"] == "nonpositive-rejected" and r["before"] != "nonpositive-rejected": return "newly-rejected"
    return None
def table(rs):
    t = collections.defaultdict(collections.Counter)
    for r in rs:
        t[r["cohort"]]["before:" + r["before"]] += 1; t[r["cohort"]]["after:" + r["after"]] += 1
        c = classify_change(r)
        if c: t[r["cohort"]][c] += 1
    return {k: dict(sorted(v.items())) for k, v in t.items()}

def place_pairs(mech):
    d = REPO / "test-diagrams/perturbations" / mech; d.mkdir(parents=True, exist_ok=False)
    out = []
    for r in rows[mech]:
        s = srcdir[r["name"] + ".puml"]
        src = (s / f"{r['name']}.puml").read_bytes(); svg = (s / f"{r['name']}.svg").read_bytes()
        assert sha(src) == r["source_sha256"] and sha(svg) == r["java_svg_sha256"]
        (d / f"{r['name']}.puml").write_bytes(src); (d / f"{r['name']}.svg").write_bytes(svg)
        ok = (r["after"] == "match") if r["java_valid"] else (r["after"] == "nonpositive-rejected")
        out.append(dict(source=f"test-diagrams/perturbations/{mech}/{r['name']}.puml", golden=f"test-diagrams/perturbations/{mech}/{r['name']}.svg",
            result="pass" if ok else "fail", axes=r["axes"] or [r["cohort"]], cohort=r["cohort"], java_valid=r["java_valid"],
            before=r["before"], after=r["after"], change=classify_change(r)))
    if mech == SP:
        for inc in sorted((R / "fresh").glob("mk2_*.inc")):
            shutil.copyfile(inc, d / inc.name)
    return out

COMMANDS = [
    "git archive 93bf53e18 / 5cf1eac12 into private dirs; CARGO_TARGET_DIR=<private>/target-{before,after} cargo build --release -j 6 -p rustuml",
    "rustc --edition=2024 -O docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs (sha256 d1424469...) --extern rustuml_oracle=<private>/target-before/release/deps/librustuml_oracle-*.rlib",
    "python3 -I support/prepare.py fresh AS=<relay>/sprite-claude-maker-batch2 AQ=<relay>/sprite-claude-batch1 F790=<cache 75ed0e64> F28=<cache 75ed0e64>/supplement; prepare.py fresh-at AT=<relay>/sprite-ownership-checker-batch1",
    "python3 -I support/replay.py LABEL MANIFEST rustuml-{before,after} compare_cli OUT CWD (svg/ast/ascii/yaml with -pipe; RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100; PLANTUML_STDLIB pinned; 6 workers) for fresh 915, AT 45, retained 4174, golden 12550",
    "python3 -I support/{classify,fresh_classify,model_changes}.py on each before/after pair",
    "python3 -I support/rejection_contract.py <fresh> fresh-manifest.json rustuml-before rustuml-after rejection-contract.json (unchanged, sha256 09cd998b...)",
    "CARGO_TARGET_DIR=<private>/target-gates cargo test --release -j 6 --workspace --lib; cargo test --release -j 6 -p rustuml --test pipe_mode; cargo test --release -j 6 --no-fail-fast -p rustuml-oracle --test golden_no_oracle --test golden_pairs --test parity_perturbations --test harness_guards -- --nocapture (on 5cf1eac12 before checker files; same oracle tests on the 93bf53e18 export; golden_no_oracle + golden_pairs on the 9834a7194 export)",
]
REVIEWER = {"task_id": "T3 thread t3-991a0cd7, branch review/sprite-ownership-claude", "model": "claude-opus-5-5"}

def write(mech, record, findings, account_commit, impl_commit, perts, extra_files):
    doc = REPO / "docs/parity-reviews" / mech; ev = doc / "checker-claude"; ev.mkdir(parents=True, exist_ok=False)
    copies = {}
    for rel, srcp in extra_files.items():
        dst = ev / rel; dst.parent.mkdir(parents=True, exist_ok=True); shutil.copyfile(srcp, dst); copies[rel] = sha(dst.read_bytes())
    record = dict(record, evidence_files=copies)
    (ev / "review.json").write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")
    guard = dict(schema_version=1, mechanism=mech, account_commit=account_commit, implementation_commit=impl_commit, java_revision=JAVA,
        reviewer=REVIEWER, commands=COMMANDS, findings=findings, verdict="rejected",
        record="checker-claude/review.json (full record; sha256 %s)" % sha((ev / "review.json").read_bytes()),
        perturbations=[{k: p[k] for k in ("source", "golden", "result", "axes")} for p in perts])
    (doc / "review.json").write_text(json.dumps(guard, indent=1, ensure_ascii=False) + "\n")

shared = {}
for f in ["runs/fresh-before/results.json", "runs/fresh-after/results.json", "runs/at-before/results.json", "runs/at-after/results.json",
          "runs/retained-before/results.json", "runs/retained-after/results.json", "runs/golden-before/results.json", "runs/golden-after/results.json"]:
    gz = R / "pkg" / (f.replace("/results.json", ".results.json.gz")); gz.parent.mkdir(parents=True, exist_ok=True)
    with gzip.GzipFile(gz, "wb", mtime=0) as fh: fh.write((R / f).read_bytes())
    shared[gz.relative_to(R / "pkg").as_posix()] = gz
for f in ["runs/fresh-changes.json", "runs/at-changes.json", "runs/retained-changes.json", "runs/golden-changes.json",
          "runs/fresh-model-changes.json", "runs/retained-model-changes.json", "runs/golden-model-changes.json", "runs/rejection-contract.json",
          "gates/no_oracle_failure_names-9834a7194.txt", "gates/no_oracle_failure_names-before.txt", "gates/no_oracle_failure_names-after.txt",
          "gates/golden_failure_names-9834a7194.txt", "gates/golden_failure_names-after.txt", "gates/perturbation-failures-before.txt",
          "gates/perturbation-failures-after.txt", "gates/guards-before.txt", "gates/guards-after.txt", "gates/status.txt",
          "batch1-local-observations.json", "fresh-manifest.json", "fresh-at-manifest.json", "retained-manifest.json", "golden-manifest.json"]:
    shared[f] = R / f
for f in ["gates/lib-after.log", "gates/pipe-after.log", "gates/oracle-after.log", "gates/oracle-before.log", "gates/oracle-base.log"]:
    gz = R / "pkg" / (f + ".gz"); gz.parent.mkdir(parents=True, exist_ok=True)
    with gzip.GzipFile(gz, "wb", mtime=0) as fh: fh.write((R / f).read_bytes())
    shared[f + ".gz"] = gz
for f in sorted(p for p in (R / "support").glob("*") if p.is_file()): shared["support/" + f.name] = f
for f in ["reference-manifest.json", "reference-input-bundle.json.gz", "SHA256SUMS"]: shared["request-batch1/" + f] = R / "batch1" / f
shared["bindings.json"] = R / "pkg/bindings.json"
(R / "pkg/bindings.json").write_text(json.dumps(dict(commits=C, java_revision=JAVA,
    jar_sha256="aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053", golden_submodule="9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a",
    toolchain="rustc 1.99.1 (58cf8f9c3 2026-10-13); cargo 1.99.1", binaries={n: sha((R / "bin" / n).read_bytes()) for n in ("rustuml-before", "rustuml-after", "compare_cli")},
    compare_cli_rs="d142446956a931cfbea5bc6bb15b844fcb9d276bf5d81399aafe2203c08c6701",
    java_references={"AS": dict(path="/Users/marcelo/Library/Caches/rustuml-relay-parent/sprite-claude-maker-batch2/reference-results.json", sha256="83586c81d9322a4b1aaea40d06bd7000330b2a4365de73ff444c1d06a5b5a1cb"),
        "AQ": dict(path="/Users/marcelo/Library/Caches/rustuml-relay-parent/sprite-claude-batch1/reference-results.json", sha256="a023e9b64410bfdeb366248fd885ee0ad41c33e8d8afbb3acb9d2c28509d062e"),
        "AT": dict(path="/Users/marcelo/Library/Caches/rustuml-relay-parent/sprite-ownership-checker-batch1/reference-results.json", sha256="98671af0cd44fc70187857a672165bf2994b047d5e5f7bf30d50e3a92b194159",
                   gz_sha256="1a061a58f1d4f9584307a4180dd0c8cfa303a20cffc56237a4b7c2505e9923ca", manifest_sha256="bd13a2c644a2df93ce99e9686818640598d418a0b6b1bb0233010cc5229e86b0",
                   bundle_sha256="d3924c7e8b389e6a039e200c2f70ebb9eae8525a4068c34bedde9b5bc9cdf2c9", transport_sha256="4d462ab69529a94f896d4904b8dd62a80b55821cd2031f69df349c4db4eef407",
                   parent_validation_sha256="2cebcde30685e6dc626db5c58e43c6e8469e418721034cfe9b1ec23f02c37e97"),
        "F790/F28": dict(path="/Users/marcelo/Library/Caches/rustuml-sequence-sprite-checker-75ed0e64 (+ supplement)")},
    environment=dict(RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100", PLANTUML_STDLIB="/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib", jobs=6)), indent=1) + "\n")

out = {}
for mech in (SP, PA):
    out[mech] = place_pairs(mech)
json.dump(dict(commits=C, tables={m: table(rows[m]) for m in (SP, PA)},
               changes={m: [dict(name=r["name"], cohort=r["cohort"], change=classify_change(r), before=r["before"], after=r["after"], after_stderr=r["after_stderr"][-300:]) for r in rows[m] if classify_change(r)] for m in (SP, PA, None)},
               perturbations={m: out[m] for m in out}), open(R / "pkg/mechanism-tables.json", "w"), indent=1)
shared["mechanism-tables.json"] = R / "pkg/mechanism-tables.json"
print(json.dumps({m: table(rows[m]) for m in (SP, PA, None)}, indent=1))
records = json.loads((R / "records.json").read_text())
for mech, acc, impl in ((SP, C["sprite_account"], C["sprite_impl"]), (PA, C["part_account"], C["part_impl2"])):
    rec = records[mech]
    rec.update(schema_version=1, mechanism=mech, verdict="rejected", reviewer=REVIEWER, account_commit=acc, implementation_commit=impl, java_revision=JAVA,
               commits=C, commands=COMMANDS, fresh_tables=table(rows[mech]), perturbations=out[mech])
    write(mech, rec, rec["findings"], acc, impl, out[mech], shared)
print({m: len(v) for m, v in out.items()})
