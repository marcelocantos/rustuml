# Independent sequence command-contract repair review

**Rejected.** All six per-mechanism reviews retain a whole-output rejection.
No new lost match, lost rejection contract, valid CLI error, or CLI panic was
found. The narrow CREATE signature-scope repair passes all 32 fresh controls;
the source-citation correction is supported. Neither observation waives the
retained failures or transfers historical acceptance.

The checker is a new independent GPT-6 Astra task, high reasoning effort,
Codex harness in T3 Code, isolated checkout `t3-2b056ca1`, branch
`review/sequence-command-contract-independent-20261105`. No children, extra
worktrees, production edits, target/baseline/comparator/skip/allow-list edits,
local Java/javac, SSH, new reference runners, pushes, PRs or releases were used.
Exact task identity is recorded in the six reviews and `findings.json`.

Baseline evidence: `9784e412146e3ece634d88ea0a1bddd6b42a82b6`, source-identical
to production `d491d4bda47faf90d4751484d45aa1387fb1b290`.
Final evidence: `dc438fd0cd0f5edd44816db81f0d765bf48ce4e6`, source-identical
to production `b4849a5008ff28c360fefe0176ee6ad073c02fd7`.
Private release targets were independently compiled at both evidence revisions
with jobs 4. All crate source files and binaries are SHA256-bound in the build
records. The unchanged `compare_cli.rs` was compiled once against the baseline
oracle library and copied byte-identically for the final replay.

## Actual CLI evidence

Every row is retained, including duplicates and nonpositive references.
The 3,111-entry replay has 3,059 distinct source byte strings and 52 duplicate
entries. The 422 fresh references have 422 distinct source byte strings, none
identical to any preserved prior checker or maker source. The fresh matrix
covers the registered sequence command families, both engines, all requested
aliases and note shapes, quoted/space identities, nested CREATE across all
eight kinds, VT/FF/global signature context, TAB/NBSP/multiple-space entries,
and the source-enumerated composite character classes.

| Cohort | Entries | Before match / mismatch / valid error / nonpositive | After |
|---|---:|---|---|
| Frozen | 2,549 | 1809 / 639 / 2 / 99 | 1809 / 639 / 2 / 99 |
| Prior checker | 534 | 348 / 116 / 10 / 60 | 370 / 104 / 0 / 60 |
| Maker controls | 24 | 4 / 12 / 2 / 6 | 12 / 6 / 0 / 6 |
| Signature supplement | 4 | 4 / 0 / 0 / 0 | 4 / 0 / 0 / 0 |
| Retained total | 3,111 | 2165 / 767 / 14 / 165 | 2195 / 749 / 2 / 165 |
| Independent fresh | 422 | 190 / 164 / 2 / 66 | 232 / 124 / 0 / 66 |

The retained replay exactly reproduces every maker CLI and library comparison
status, including all 16 alias matches and four signature supplement matches.
It has 30 whole-output gains. The fresh matrix has 42 whole-output gains and
recovers two valid-input CLI errors on empty across notes; the legacy empty
note still mismatches. Both cohorts have zero lost matches in the shipped CLI,
zero lost negative rejection contracts, and zero new valid CLI errors or
panics. Fresh Java-valid references number **356**, with **232 matches and 124
mismatches**. Nonpositive references are never counted as matches.

All 2,549 frozen AST, ASCII and YAML **stdout bytes** are identical before and
after. Across 3,111 entries, 58 AST, 36 ASCII and 52 YAML stdout streams change,
reproducing the maker's diagnostics. The same 35 pre-existing YAML serialization
panics remain; none is newly introduced. Complete binary stdout/stderr streams
are retained as base64. This does not assert ASCII parity or equivalence of
panic stderr locations. Fresh YAML behavior was not claimed or substituted
for SVG evidence.

## Rejections and reference classification

The unchanged `rejection_contract.py` was executed on explicit, separately
preserved manifests:

- Historical: 77/99 rejected before and after; 22 false successes remain.
- Prior fresh syntax cases: 36/58 rejected before, 40/58 after; all four accross
  rejections recover, 18 false successes remain.
- Eight retained exporter failures: all Rust exit 0, kept separate from syntax
  rejection obligations and never counted as SVG passes.
- Independent fresh command rejections: 0/54 rejected in both Rust revisions;
  all 54 remain false successes. Of these, 48 are syntax/command rejections
  and six are color execution errors. There are no lost rejection contracts.
- Twelve independent fresh **exit-zero exporter failures** remain separate.

The first reference batch has 312 valid outputs, 34 exit-200 command rejections
and 12 exit-zero exporter failures. Of the 34 rejections, 32 also fail while
exporting their error diagnostic because it contains a control character.
The supplement has 44 valid outputs, 14 analogous command rejections with
failed error-diagnostic export, and six color errors. Thus the final taxonomy
is 356 valid, 46 command rejections with failed diagnostic export, two ordinary
syntax rejections, six color errors, and 12 exit-zero exporter failures.
The export failure on an already rejected command does not erase its rejection
obligation. The automated import's initial status-based classification is
preserved; `fresh_summary.py` records the six color refinements separately.

All 30 intended-versus-observed disagreements are retained without rewriting
intent. Four predicted-invalid VT cases are Java-valid: `CommandBoxEnd` and
`CommandFootbox` use `spaceZeroOrMore`, so they do not carry a required-space
signature bit. Rust's end-box tokenization handles VT (legacy passes; Teoz
retains an envelope mismatch). Rust's literal `hide footbox` check misses VT
in both revisions. The six predicted-valid `#back:LightBlue: cargo` note cases
report **No such color**: `ColorParser.PART2` cannot end before the following
colon, the alternative captures `#back`, `Colors` strips `#`, and
`HColorSet#getColor("back")` throws `NoSuchColorException`.
`SingleLineCommand2#execute` catches it as `badColor`. The source-extracted
capture derivation is retained alongside raw Java stdout/stderr.

## Mechanism findings

| Mechanism | Fresh matches / entries | Whole-output verdict |
|---|---:|---|
| Required note identity | 46/56 | Rejected: 10 valid shape/envelope mismatches |
| FoxSignature whitespace | 103/224 | Rejected: 63 valid mismatches, 58 nonpositives |
| Command entry grammar | 21/42 | Rejected: 19 valid mismatches, two syntax negatives |
| Pattern2 composite classes | 30/68 | Rejected: 32 valid mismatches, six color errors |
| CREATE signature scope | 32/32 | Supported locally; packet rejected with retained Display/geometry failures |
| Citation correction | Two duplicate passing witnesses | Source correction supported; no behavioral acceptance transferred |

The note grammar now requires identities and can backtrack over the optional
`of` prefix. No Java-valid fresh note form errors. There is no fixture-label
branch. The whitespace table is derived from `FoxSignature`, including its
omission of VT; it is not a VT fixture exception. No non-VT input newly rejects.
The CREATE call is the only recursive use of the guarded declaration parser;
other tail consumers and unsupported inline-arrow aliases are distinguished
in `source-audit.json`.

Each account precedes its implementation and is unchanged. All 21 maker-cited
Java files are byte-equal to pinned source. The corrected declaration methods
and `DrawableSetInitializer#prepareParticipant` really exist. However, the
**new entry account and parser comment cite nonexistent
`CommandAutonumberResume#getRegexConcat`; its method is `getConcat`**.
This needs new append-only corrective evidence, not an account rewrite.

The composite-class source audit finds Java's additional autonumber class
`[^%pLN%s]`. Its punctuation/whitespace variants remain executable; Rust's
narrower autonumber grammar is not silently blessed. Raw first-run failure-name
files confirm 29 colored-note losses in both main tiers. Raw candidate-two
measurements confirm two CREATE signature supplement losses. The independent
final build has neither: both main failure-name files exactly equal baseline,
and all four signature supplements match.

## Broad gates and preservation

Stable broad gates ran **once per tested revision**, before registering new
checker pairs. Final units: parser **586**, renderer **800**, CLI **13**.
Main **library diagnostics**: product **11181/11251**, assisted **10497/11251**,
sequence **1411/1411**. Main failure-name files were copied immediately after
each revision's gate and are byte-identical. These are not 11,251 CLI processes.

Final standing perturbations remain red at **1401/2258**; guards are **4/6**,
with decimal provenance and review-acceptance gates red. Baseline standing was
**1031/1724**; those differing corpus sizes are not a regression comparison.
Owned formatting passes. Workspace class formatting, parser component dead
code, and the 19 existing renderer clippy findings remain red.

Initial golden hydration failed because file transport was disabled. Its log
is preserved. The explicit local-file transport retry succeeded, and the
submodule is exactly `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`.
No broad gate was rerun after adding evidence. The new 422 inputs are already
executable through the unchanged standing collector and have separate actual
CLI and library comparison captures. Two byte-identical provenance witness
copies satisfy the review schema's per-mechanism path rule: **424 new pair
paths, 422 new distinct inputs**, bringing registered standing paths to 2,682.
These duplicate registrations are not added to the 422-case measurement.
The staged whitespace check reports six deliberate trailing-whitespace lines
in spacing probes; they are preserved byte-for-byte as required. Its log is
retained. New review shapes and every paired source/SVG hash were validated separately;
this is not a claim that the red repository guards passed.

All **10,159 prior evidence paths** and **1,641 imported evidence paths** are
byte-exact in both Git objects and this checkout. All old reviews remain.
Only sequence parser behavior and one sequence-renderer comment differ between
the tested production revisions. This checker changes evidence only.

## Boundaries and reproduction

The six maker failures (two quoted activation WHO, four shaped across), eight
nested Teoz boxes, one created-control case, 22 note-geometry cases, T14.17,
T14.20, short question arrows, T14.13 headings, logical fonts, frozen failures
and TIM welcome remain explicit failures. Their continued failure is not
attributed to this packet without before/after causal evidence.

Next work is bounded to the missing method citation, command rejection
propagation and the retained geometry/Display/command-language dependencies.
All 124 fresh valid mismatches and 66 nonpositive cases remain preserved;
none is parked or waived. No combined correctness is claimed for the concurrent
shared SVG serialization maker.

New Java references used only the parent's existing Monkey Man relay, with
source revision `71806a23780b04a5ccde2f8ceb5121edad5eb711` and JAR SHA256
`aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
Both batches independently validate exact source/stdin bytes, hashes, JSON-array
argv, private cwd substitution, GMT+11:00, env {}, headless UTF-8/en_AU,
response/transport hashes and original bundle fields. There were no timeouts.
Java argv is derived from the pinned wrapper, not process-traced. Historical
OpenJDK 21.0.5 / Graphviz 12.2 equivalence remains unproved.

`captures.tar.gz` contains full CLI SVGs, model/ASCII/YAML streams, library
diagnostics, immutable reference envelopes, commands, logs, failure-name
snapshots and setup failures; `archive.json` binds it. Compact summaries,
source audit, review index and rejection manifests are alongside this README.

To reproduce, copy `support/` scripts into a **new private cache**, never run
capture-producing copies in this evidence directory. Use jobs 4, a private
`CARGO_TARGET_DIR`, the recorded stdlib path, and no baseline-write variables.
The unchanged runner is
`docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py`:
`uv run --no-project python <runner> MANIFEST BIN_DIR OUTPUT_DIR`.
Use the existing maker `all-inputs.json` for 3,111 entries, and decompress
`fresh-all.json.gz` for the 422-case manifest. Compare actual CLI output with
the unchanged `compare_cli.rs`; keep `diff_one` library diagnostics separate.
Replay the preserved reference pairs; do not regenerate them locally.
