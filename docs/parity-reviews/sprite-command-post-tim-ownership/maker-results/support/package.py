"""Package the Claude checker evidence: perturbation pairs, per-mechanism review.json records, summaries.

Usage: python3 -I package.py CACHE REPO
Writes only new paths; refuses to overwrite anything that already exists.
"""
import gzip, hashlib, json, re, shutil, sys, tarfile
from pathlib import Path

C, REPO = Path(sys.argv[1]), Path(sys.argv[2])
sys.path.insert(0, str(C / "support"))
from classify import outcome, load  # noqa: E402

EV_REL = "docs/parity-reviews/sequence-sprite-participant-packet/checker-claude"
EV = REPO / EV_REL
sha = lambda b: hashlib.sha256(b).hexdigest()


def new_file(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        raise SystemExit(f"refusing to overwrite {path}")
    path.write_bytes(data if isinstance(data, bytes) else data.encode())


def jdump(obj):
    return json.dumps(obj, indent=1, ensure_ascii=False) + "\n"


REV = dict(
    tested_rust_revision="f4ff1cb4e0678fee8f099c14c92df07529b0afbb",
    production_revision="ba2e6c2b79bb35841351ce2ed2e88e3a7abbe9d7",
    baseline_revision="c6295ccdef39719b60f0e88dfd9555919523c961",
    baseline_production_revision="c525fd32ae97b642b440a161140e9457f29bcfc8",
    java_revision="71806a23780b04a5ccde2f8ceb5121edad5eb711",
    jar_sha256="aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053",
)
REVIEWER = dict(
    task_id="t3-worktree:t3-6ce8db92 review sub-agent (sprite/participant packet checker)",
    model="claude-opus-5-5",
    tool="Claude Code (Claude Agent SDK) in T3 Code",
    checkout="/Users/marcelo/.t3/worktrees/rustuml/t3-6ce8db92",
    branch="review/sprite-participant-claude",
    independent=True,
    prior_involvement=False,
    children=0,
)
COMMANDS = [
    "git archive c525fd32ae97b642b440a161140e9457f29bcfc8 | tar -x -C PRIVATE/src-before; git archive ba2e6c2b79bb35841351ce2ed2e88e3a7abbe9d7 | tar -x -C PRIVATE/src-after",
    "CARGO_BUILD_JOBS=4 CARGO_TARGET_DIR=PRIVATE/target-{before,after} cargo build --release -j 4 -p rustuml -p rustuml-oracle --bin rustuml --example diff_one (in each exported tree)",
    "rustc --edition=2024 -O docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs -L dependency=PRIVATE/target-before/release/deps --extern rustuml_oracle=PRIVATE/target-before/release/deps/librustuml_oracle-9463d30e464f1e2a.rlib -o PRIVATE/bin/compare_cli",
    "python3 -I support/replay.py LABEL MANIFEST PRIVATE/bin/rustuml-{before,after} PRIVATE/bin/compare_cli OUT CWD  (modes -tsvg/--ast/-ttxt/--yaml with -pipe; RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100; PLANTUML_STDLIB pinned) for fresh 818 (cwd=private reference dir), retained 4174 and golden 12550 (cwd=repo root)",
    "python3 -I docs/parity-reviews/sequence-consumer-contract-packet/checker-9784-astra-b4b6/support/rejection_contract.py PRIVATE/fresh fresh-manifest-classified.json PRIVATE/bin/rustuml-before PRIVATE/bin/rustuml-after rejection-fresh.json",
    "CARGO_TARGET_DIR=PRIVATE/target-gates cargo test --release -j 4 --lib -p rustuml-parser -p rustuml-render; cargo test --release -j 4 -p rustuml --test pipe_mode; cargo test --release -j 4 --no-fail-fast -p rustuml-oracle --test golden_no_oracle --test golden_pairs --test parity_perturbations --test harness_guards -- --nocapture (once, on f4ff1cb4e before any checker file was added)",
    "Java references: staged by the previous (quota-terminated) checker in ~/Library/Caches/rustuml-sequence-sprite-checker-75ed0e64 (790 + 28 cases, parent Monkey Man relay, JSON-array argv, PRIVATE_CWD, GMT+11:00). Validity classified here from raw SVG + stderr. No local Java/javac/SSH.",
]

manifest = json.loads((C / "fresh-manifest-classified.json").read_text())
before = {r["name"]: r for r in load(C / "runs/fresh-before/results.json")}
after = {r["name"]: r for r in load(C / "runs/fresh-after/results.json")}


def family(svg_bytes):
    m = re.search(rb'data-diagram-type="([A-Z_]+)"', svg_bytes)
    return m.group(1).decode() if m else None


# ---------------------------------------------------------------- perturbations
per_mech = {}
for case in manifest:
    mech = case["mechanism"]
    name = case["name"]
    dest = REPO / "test-diagrams/perturbations" / mech
    src = (C / "fresh" / case["source"]).read_bytes()
    svg = (C / "fresh" / case["golden"]).read_bytes()
    new_file(dest / f"{name}.puml", src)
    new_file(dest / f"{name}.svg", svg)
    ob, oa = outcome(before[name]), outcome(after[name])
    valid = case["observed_valid"]
    passed = (oa == "match") if valid else (after[name]["results"]["svg"]["exit"] != 0)
    per_mech.setdefault(mech, []).append(dict(
        source=f"test-diagrams/perturbations/{mech}/{name}.puml",
        golden=f"test-diagrams/perturbations/{mech}/{name}.svg",
        axes=case["axes"],
        result="pass" if passed else "fail",
        result_semantics="whole actual-CLI output via unchanged compare_cli for Java-valid; explicit CLI rejection for Java-invalid; validity from raw Java SVG+stderr",
        source_sha256=sha(src), golden_sha256=sha(svg),
        intended_valid=case["intended_valid"], observed_valid=valid,
        java_exit=case["java_exit"], java_family=family(svg),
        cli_exit_before=before[name]["results"]["svg"]["exit"], cli_exit_after=after[name]["results"]["svg"]["exit"],
        comparator_exit_before=before[name]["comparison"]["exit"], comparator_exit_after=after[name]["comparison"]["exit"],
        outcome_before=ob, outcome_after=oa,
        rust_warning_after=bool("warning" in after[name]["results"]["svg"]["stderr"]),
    ))
for inc in sorted((C / "fresh").glob("*.inc")):
    new_file(REPO / "test-diagrams/perturbations/sequence-sprite-pending-span" / inc.name, inc.read_bytes())


def counts(entries):
    valid = [e for e in entries if e["observed_valid"]]
    inv = [e for e in entries if not e["observed_valid"]]
    tally = lambda side: {k: sum(e[f"outcome_{side}"] == k for e in valid) for k in ("match", "mismatch", "valid-error")}
    return dict(
        entries=len(entries), java_valid=len(valid), java_invalid=len(inv),
        valid_before=tally("before"), valid_after=tally("after"),
        gains=sum(e["outcome_before"] != "match" and e["outcome_after"] == "match" for e in valid),
        lost_matches=sum(e["outcome_before"] == "match" and e["outcome_after"] != "match" for e in valid),
        new_valid_errors=sum(e["outcome_before"] != "valid-error" and e["outcome_after"] == "valid-error" for e in valid),
        invalid_rejected_before=sum(e["cli_exit_before"] != 0 for e in inv),
        invalid_rejected_after=sum(e["cli_exit_after"] != 0 for e in inv),
        lost_rejections=sum(e["cli_exit_before"] != 0 and e["cli_exit_after"] == 0 for e in inv),
        intention_disagreements=sum(e["intended_valid"] != e["observed_valid"] for e in entries),
        panics=0, timeouts=0,
    )


def names(mech, pred):
    return sorted(Path(e["source"]).stem for e in per_mech[mech] if pred(e))


# ---------------------------------------------------------------- review records
SPRITE_REGRESSIONS = names("sequence-sprite-block-consumption",
                           lambda e: e["observed_valid"] and e["outcome_after"] == "valid-error" and e["outcome_before"] != "valid-error")
PENDING_REGRESSIONS = names("sequence-sprite-pending-span",
                            lambda e: e["observed_valid"] and e["outcome_after"] == "valid-error" and e["outcome_before"] != "valid-error")
UNCLOSED_ACCEPTED = names("sequence-sprite-completion-admission", lambda e: not e["observed_valid"] and e["cli_exit_after"] == 0)

COMMON_LIMITS = [
    "Java argv is wrapper-derived (parent relay), not process-traced; remote OpenJDK 21.0.5 environment recorded by the staging checker.",
    "No production, comparator, baseline, skip-rule, allow-list, park-list, census or target change was made by this checker.",
    "Library-tier gates (golden_no_oracle/golden_pairs/parity_perturbations) are diagnostics; actual CLI replays are the truth used here.",
    "Additional Java references for case-widened single-line sprite grammar and inline-comment-stripped SVG headers were requested (batch1, see pending-request/); their local before/after CLI observations are recorded but carry no Java verdict yet.",
]

reviews = {
    "sequence-sprite-block-consumption": dict(
        account_commit="589b3d5e656a9a02b8e03f9f4fd103be084f2bef",
        implementation_commits=["d15d6eaf928d1b3eaf6aa49c9cf4d57b69e9caad", "ba2e6c2b79bb35841351ce2ed2e88e3a7abbe9d7"],
        findings=[
            "REJECT: new valid-input errors outside the approved 15-entry exception set. Java-valid programs whose unclosed sprite header lies in an inactive !if branch or in an uninvoked !procedure body (c75s_inactive_unclosed_{svg,pixel}_{legacy,teoz}, c75s_macro_uninvoked_unclosed_{svg,pixel}_{legacy,teoz}) now exit 1. Four of them were whole-output matches at baseline (both SVG variants, both modes): lost matches. Sources: " + ", ".join(SPRITE_REGRESSIONS) + ".",
            "Cause (source-traced): PreprocessContext::process_one_line evaluates the sprite body branch and the declaration block before block-comment, @start/@end, !startsub/!function/!procedure collection, !if/is_active gating and !include expansion. Java runs the TIM preprocessor first, so CommandMultilines2#isValid and PSystemCommandFactory#getCandidate never see inactive or uninvoked lines. The packet's deferred blanking now pushes the raw pending line (output.push(line.to_owned())) for every line until END, bypassing is_active; the raw 'sprite ...', 'FF' and '!endprocedure' lines reach strict sequence admission.",
            "Locally correct: SVG END (.*</svg>)$ and pixel END ^end[%s]?sprite|\\}$ with Matcher.matches on StringUtils#trin-trimmed lines, ASCII case folding and %s (incl. VT, NBSP) mirror Pattern2/CommandFactorySprite/CommandSpriteSvgMultiline; inline CommandSpriteSvg precedes the multiline command as in CommonCommands#addCommonCommands2. Fresh valid sprite cases: 14 baseline errors recover (nested/attribute/back-to-back/if_true/last/procedure), 4 mismatches become matches (end-sprite forms). bd087_reg_27_plain recovers.",
            "Not every family: unused/used sprites in class/state/activity/usecase/component remain whole-output mismatches for pixel use (c75_use_pixel_*: 7 silent mismatches); sprite rendering remains unsupported (explicit warnings for SVG).",
            "Pending Java verdict (batch1): the declaration grammar is applied context-free to every line, now case/separator-insensitive. Locally, 'Sprite sheets are loaded' inside a sequence/class/state/activity note, legend, ref or class body, and class links such as 'Sprite -- Player' (ClassDiagramFactory registers CommonCommands2 after CommandLinkClass) are blanked after the packet but rendered before it; 'Sprite frame {' inside a note opens a pixel block and now exits 1. Not counted as confirmed regressions until Java references return.",
        ],
        obligations=[
            "Restore TIM ordering for sprite ownership: only active, expanded lines (after !if/!procedure/!include handling) may open, extend or close a sprite block; an incomplete block in an inactive branch or uninvoked macro must be invisible, as in Java. Perturbations: inactive/uninvoked unclosed SVG and pixel headers, include inside a body, legacy and teoz.",
            "Bound declaration matching to command context: lines inside multiline command bodies and lines a family factory dispatches before CommonCommands must not be consumed as sprite commands. Await batch1 Java references before choosing the mechanism.",
        ],
    ),
    "sequence-sprite-completion-admission": dict(
        account_commit="0d9e70030847b1479275b26b8965c8735d172cfe",
        implementation_commits=["d15d6eaf928d1b3eaf6aa49c9cf4d57b69e9caad", "ba2e6c2b79bb35841351ce2ed2e88e3a7abbe9d7"],
        findings=[
            "REJECT: the claimed every-family admission is not achieved. Ten Java-invalid unclosed sprite programs in class, state, activity, usecase and component diagrams still exit 0 after the packet (false successes, unchanged from baseline, not lost rejections): " + ", ".join(UNCLOSED_ACCEPTED) + ".",
            "Sequence: six previously accepted Java-invalid incomplete SVG/pixel blocks (legacy/teoz) are now rejected; zero lost rejections. Diagnostics differ from Java (Syntax Error vs CommandSpriteFile 'Cannot read'); no diagnostic equivalence is claimed.",
            "The retention of incomplete commands is the direct cause of the block-consumption regression: retained raw lines bypass is_active, so an incomplete header in an inactive !if branch or uninvoked !procedure (Java: invisible) is now admitted as a command and rejected.",
        ],
        obligations=[
            "Make incomplete-command visibility depend on TIM activity (active, expanded lines only) and extend admission to the non-sequence families that register CommonCommands sprite commands, with class/state/activity/usecase/component negative controls.",
        ],
    ),
    "sequence-sprite-pending-span": dict(
        account_commit="2268b0ddf9daf5752b885b14497dc80e7ea4e5fd",
        implementation_commits=["ba2e6c2b79bb35841351ce2ed2e88e3a7abbe9d7"],
        findings=[
            "REJECT: two new valid-input errors: c75_include_pixel_into_include_{legacy,teoz} (Java-valid; pixel body rows and closing brace supplied by !include) were baseline mismatches and now exit 1 ('no registered sequence command accepts \"!include c75_rows.inc\"'). Java expands the include during TIM, before CommandFactorySprite sees the body; Rust treats the !include line as raw body text.",
            "Four Java-valid include cases remain errors: c75_include_parent_to_child_{legacy,teoz} and c75_include_sibling_{legacy,teoz} (SVG opened in the parent or one include and closed in another).",
            "Locally correct: the pending-suffix count fixes the absolute-index erasure; six include cases (closed, child_to_parent, nested; legacy/teoz) move from baseline errors to whole matches. The new unit test proves preceding declarations survive.",
        ],
        obligations=[
            "Expand !include (and other TIM directives) before sprite body ownership so a body can span include boundaries in either direction; perturbations: parent-to-child, sibling, pixel rows from include, legacy/teoz.",
        ],
    ),
    "sequence-participant-complete-consumption": dict(
        account_commit="589b3d5e656a9a02b8e03f9f4fd103be084f2bef",
        implementation_commits=["d15d6eaf928d1b3eaf6aa49c9cf4d57b69e9caad"],
        findings=[
            "REJECT for whole-output parity; no regression found. Eighteen compact declarations (touching stereotype/colour, all nine keywords incl. create, legacy/teoz) gain whole matches; zero lost matches, new errors or lost rejections; 72 Java-invalid order forms stay rejected.",
            "The invariant 'unsupported variants produce explicit residue, never silent truncation' is violated: 42 Java-valid fresh declarations mismatch with no warning. Multiple stereotypes (<<woody>><<tall>>) capture as one label and render « woody>><<tall » (Java's (<<.+?>>) capture is the same, but Stereotype#build splits it); URL tooltips ([[url{tip} label]]) are dropped silently. These predate the packet but disprove completeness.",
            "Re-declaration with a box (c75s_participant_redeclare_box_*) moves from error to mismatch; Java CommandParticipant#executeArg early return calls SequenceDiagram#putParticipantInLast and records the englober, which Rust does not model.",
            "Regex audit: identity [%pLN_.@]+, A/A2/A3/A4 alternatives, StereotypePattern.optional spacing, ORDER -?\\d{1,7} before URL/COLOR and end anchor match the pinned CommandParticipantA..A4/CommandParticipant sources.",
        ],
        obligations=[
            "Execute or explicitly report multi-stereotype lists and URL tooltip/label parts; model or warn for putParticipantInLast/englober on re-declaration.",
        ],
    ),
    "sequence-participant-residue-preservation": dict(
        account_commit="643f125a81867c6c492a59fd468346e31b18c3b1",
        implementation_commits=["d15d6eaf928d1b3eaf6aa49c9cf4d57b69e9caad"],
        findings=[
            "REJECT for whole-output parity; no regression found. All 42 Java-valid signed-order/default-zero probes mismatch before and after; 40 carry the new explicit signed-order warning. participant_negative_legacy keeps its baseline match (retained cohort).",
            "Silent residue: c75s_participant_zero_order_{legacy,teoz} re-declare an existing participant with 'order 0'; Java's early return still calls putParticipantInLast (moves the participant to last), Rust's new existing-participant early return skips the command without a warning.",
        ],
        obligations=[
            "Model signed ORDER with Java's stable insertion (SequenceDiagram#addWithOrder) and the re-declaration reorder, or report each as explicit residue.",
        ],
    ),
    "sequence-command-specific-color-paths": dict(
        account_commit="589b3d5e656a9a02b8e03f9f4fd103be084f2bef",
        implementation_commits=["d15d6eaf928d1b3eaf6aa49c9cf4d57b69e9caad"],
        findings=[
            "REJECT for whole-output parity; no regression found. Five participant/create composite COLOR tokens that Java rejects via HColorSet#getColor are now rejected (179/179 Java-invalid rejected after, 174 before); two baseline errors recover (repeat early return, multiline unused COLOR); zero lost matches or rejections.",
            "The '-' gradient condition is fixed: dash gradients now warn like slash/bar/backslash. Repeat declarations correctly skip COLOR (no warning, whole matches); multiline participants warn as unimplemented.",
            "154 Java-valid colour probes still mismatch (painting/gradient/style materialization), 24 of them without any warning (e.g. note/hnote short-hex and simple colours, group duplicate, return backslash).",
            "Early return omits Java's putParticipantInLast side effect (see participant reviews).",
        ],
        obligations=[
            "Paint participant/note/box gradients and composite Colors faithfully, or warn for every unpainted colour path (24 silent mismatches).",
        ],
    ),
}

fresh_counts_all = {}
index = []
for mech, spec in reviews.items():
    entries = per_mech[mech]
    fc = counts(entries)
    fresh_counts_all[mech] = fc
    review = dict(
        schema_version=1, mechanism=mech, reviewer=REVIEWER,
        account_commit=spec["account_commit"], implementation_commit=spec["implementation_commits"][-1],
        implementation_commits=spec["implementation_commits"], **REV,
        commands=COMMANDS, perturbations=entries, findings=spec["findings"], verdict="rejected",
        verdict_scope="Whole actual-CLI output and rejection-contract acceptance; bounded or locally correct support is not a waiver. Locally correct portions are distinguished in findings.",
        fresh_counts=fc, evidence=EV_REL, bounded_next_obligations=spec["obligations"], limits=COMMON_LIMITS,
    )
    new_file(REPO / "docs/parity-reviews" / mech / "review.json", jdump(review))
    index.append(dict(mechanism=mech, review=f"docs/parity-reviews/{mech}/review.json", verdict="rejected",
                      perturbation_entries=len(entries), failing_entries=sum(e["result"] == "fail" for e in entries)))

# ---------------------------------------------------------------- evidence files
new_file(EV / "review-index.json", jdump(index))
new_file(EV / "fresh-counts.json", jdump(fresh_counts_all))
for name in ["fresh-manifest-classified.json", "fresh-tables.json", "fresh-changes.json", "rejection-fresh.json",
             "retained-summary.json", "golden-summary.json", "retained-model-changes.json", "fresh-model-changes.json",
             "golden-model-changes.json", "perturbation-failures.txt", "retained-manifest.json"]:
    new_file(EV / name, (C / name).read_bytes())
for run in ["fresh-before", "fresh-after", "retained-before", "retained-after", "golden-before", "golden-after"]:
    new_file(EV / "runs" / f"{run}.results.json.gz", gzip.compress((C / "runs" / run / "results.json").read_bytes(), mtime=0))
for f in (C / "gate-failure-names").iterdir():
    new_file(EV / "gate-failure-names" / f.name, f.read_bytes())
for f in sorted((C / "logs").iterdir()):
    new_file(EV / "logs" / (f.name + ".gz"), gzip.compress(f.read_bytes(), mtime=0))
for f in sorted((C / "support").glob("*.py")) + sorted((C / "support").glob("*.sh")):
    new_file(EV / "support" / f.name, f.read_bytes())
for f in ["reference-manifest.json", "reference-input-bundle.json.gz"]:
    new_file(EV / "pending-request" / f, (C / "batch1" / f).read_bytes())

# raw captures: all fresh outputs; retained rows whose any mode changed; golden rows whose any mode changed
def capture_tar(label, rows_b, rows_a, only_changed):
    path = EV / f"captures-{label}.tar.gz"
    if path.exists():
        raise SystemExit(f"refusing to overwrite {path}")
    with tarfile.open(path, "w:gz") as tar:
        for x, y in zip(rows_b, rows_a):
            changed = any(x["results"][m]["stdout_sha256"] != y["results"][m]["stdout_sha256"] for m in x["results"])
            if only_changed and not changed:
                continue
            key = sha(x["source"].encode())[:24]
            for side in ("before", "after"):
                for m in ("svg", "ast", "ascii", "yaml"):
                    f = C / "runs" / f"{label}-{side}" / f"{key}.{m}"
                    tar.add(f, arcname=f"{label}-{side}/{key}.{m}")

capture_tar("fresh", load(C / "runs/fresh-before/results.json"), load(C / "runs/fresh-after/results.json"), False)
capture_tar("retained", load(C / "runs/retained-before/results.json"), load(C / "runs/retained-after/results.json"), True)
capture_tar("golden", load(C / "runs/golden-before/results.json"), load(C / "runs/golden-after/results.json"), True)
print(json.dumps(fresh_counts_all, indent=1))
