# Corrective shared-text maker packet — new review required

This corrects three bounded dependencies on clean
`246ff6396b538843143ee9d2cc9a569b67f614dd`. It does not accept TIM8 or the
previous sprite packet. Rejection `83469a4832cf20b8e2e57514841e5c46adf63a6e`
was imported unchanged by cherry-pick `6b0ca33e88127a44bad1e564544db7d746816d6d`.
All 9,731 previously inventoried files and all 118 imported files retain
exact bytes. No checker review was authored or changed by this maker.

Three separate schema-complete accounts preceded all production drafts:
`ee026189` with source-location correction `e25a4a20`:

- `docs/parity-reviews/creole-command-candidate-traversal/account.json`
- `docs/parity-reviews/creole-present-empty-stripe/account.json`
- `docs/parity-reviews/wbs-semantic-text-measurement/account.json`

Implementation is `59dc60604792260c342f6e6ab96829f10b0a59f5`, changing only
`creole.rs`, `text_render.rs`, and `wbs.rs`. Public APIs are unchanged.
Parser/model/dispatch, class/sequence consumers, TIM, metrics/fonts/styles,
native/layout, registries, comparator, baselines, skips, allow-lists, targets
and shipping state were not changed. This is maker evidence for a **new
independent review**, not a self-authored acceptance or completion attestation.

## Causal coverage

Pinned Java source is `71806a23780b04a5ccde2f8ceb5121edad5eb711`.
`java-source-provenance.json` records exact local source paths and hashes;
the accounts identify the classes, methods, invariants and perturbation axes.

`StripeSimple.searchCommand/modifyStripe` searches command candidates at the
current position; a miss consumes a literal character. Rust now examines
angle candidates with a tentative iterator and commits it only after
recognition. Unrecognized markup leaves later commands visible, preserving
UTF-8 character boundaries and existing escape/style dispatch. The parser
still retains command flush boundaries, even for unresolved commands.

`Display.getWithNewlines(null)` returns NULL, whereas an actual empty string
creates an empty line. `StripeSimple.getAtoms` supplies a base-configuration
space after nested styles restore. A private actual-line projection in
`text_render` now supplies that same atom to width, height/ascent, counts and
paint. Lexical `parse_segments("")` stays empty; absent display/label guards
remain caller-owned. The fresh no-label and empty message-label controls
still emit no message text atoms, exactly as Java does. No padding is added
to the diagram models.

Java WBS builds a Creole `SheetBlock` in `FtileBoxOld`, measures it and paints
that same component. Rust WBS now measures the same semantic label projection
it paints, using existing shared width/height/first-baseline APIs. Measured
node height replaces the plain-font constant in the existing layout equations.
Padding remains the Java `wbsDiagram Padding 10` default. No topology, spacing,
coordinate correction, registry or new WBS syntax was introduced.

The measurements separate the effects:

| Effect | Exact control | Before → after; Java |
| --- | --- | --- |
| Command traversal and flush | checker `nested_angle` | 1 → 2 text atoms; canvas width 256 → 187; Java 2 atoms/187 |
| Existing flush preservation | maker `missing_runs` | 3 → 3 text atoms, width 174; matches Java unchanged |
| Ordinary empty atom | checker `empty_lines` | blank text length 0 → 4.1133 at base size 13; Java space 4.1133; 9 total text atoms and height 184 unchanged |
| Style restoration | fresh `blank_restored` | two zero-length blanks become base-size-13 spaces, not size 24/30; whole SVG matches |
| Absent label | fresh `absent_message`, `blank_message` | 4 participant text atoms before/after/Java, no synthetic message stripe |
| WBS measurement | checker `wbs`, `legacy_wbs` | width 137 → 78, height 149 unchanged, same 4 painted text atoms; matches Java |
| WBS styled size | fresh `wbs_styles` | canvas 347×198 → 230×207; same 6 text atoms; matches Java |

`causal-coverage.json` and `text-observations.json` retain the full element
attributes and before/after/reference projections, not just these examples.
They are diagnostics, not substitutes for whole-SVG comparison.

## Actual CLI results

Every row ran the privately built original-source CLI with `-pipe`, separately
with `--yaml -pipe`, and through the unchanged strict comparator. Duplicates
and nonpositive references are retained.

| Immutable cohort | Entries | Valid references | Before | After |
| --- | ---: | ---: | ---: | ---: |
| Retained TIM, including checker8 | 1,730 | 1,609 | 709 | 709 |
| Original maker | 18 | 17 | 13 | 13 |
| Existing golden sprites | 50 | 49 | 49 | 49 |
| Rejected checker | 38 | 34 | 22 | 26 |
| Fresh corrective maker | 15 | 15 | 6 | 13 |

There are no lost passes. The 1,730 retained entries contain 1,651 distinct
source/reference pairs and 79 duplicate entries. Their SVG exit distribution
remains 1,605 exit-0, 93 exit-1, and 32 exit-101 panics. YAML retains 1,622
exit-0, 93 exit-1, and 15 exit-101 outcomes. Other cohorts' CLI SVG/YAML
commands all return 0; validity is independently determined, not inferred
from those exits. Every raw stderr, including empty streams, is archived.

All full parsed YAML trees are semantically unchanged. One existing sprite
YAML changes map serialization order only; its original bytes are retained.
All 207 available valid retained Java evaluated sources and all 17 valid
maker evaluated sources match CLI YAML source exactly. Checker38 and fresh15
are SVG-only references; no nonexistent Java SourceProbe result is claimed.
No TIM evaluation change or new valid-input TIM defect is claimed.

All four targeted checker failures now pass: `nested_angle`, `empty_lines`,
`wbs`, and `legacy_wbs`. Fresh gains cover escape handling and recognized
styles inside unknown angle text, ordinary/restored blanks including base
size 8, and WBS missing/styled/left-child text dimensions. Plain WBS and
existing defined note/deployment controls retain their passes.

## All residuals remain failures

`summary.json` enumerates every failed/nonpositive/panic membership.
`dependency-handoff.json` provides revision-specific source ownership and
original source/reference paths for both markup controls.

- Original `markup_literal` and `markup_scan` whole SVGs remain failures,
  byte-identical in this correction. This tested tree's `parse/class.rs:731-739`
  only mirrors generic splitting for labels derived from the id. The separate,
  unintegrated class candidate `68ab512be964e243ae1943f7a9fefd3537091401`
  already splits the quoted label at line 993 and has positioned-stripe caller
  work. No duplicate repair or combined correctness claim is made.
- Class `style_nested` and maker `missing_size` still fail caller placement.
  Source-line-zero renderer fallback is a separate visible dependency.
- Fresh `nested_unknown` remains a failure: the first inner command is now
  consumed correctly, but Java `CreoleParser.java:175` invokes
  `Guillemet.manageGuillemet` before the stripe parser. This tree's sequence
  message `process_label` omits that caller projection. Java retains
  `«$枝»`; Rust consumes the nested command. No sequence file was edited.
- Fresh `nested_unicode` consumes the inner command at the correct boundaries
  but remains a whole failure: Java's `Ω<β` width is 28.5073 versus Rust's
  25.3462, and Java recognizes `<plain>` through `FontStyle.PLAIN` while Rust
  preserves that unsupported tag as before. No metrics or style-reset feature
  was added to conceal this failure.
- `note_undefined` still loses the sprite-only line in its family resolver.
  Defined note scale/color, class and sequence sprite failures require their
  registry owners. These are **defined sprites**, not missing ones.
- URL label/empty-link recursion, URL/italic parsing and legacy `to_svg_tspans`
  remain unresolved. All original failed inputs remain unchanged.
- Checker scales `.` and `1..2` are Java `NumberFormatException` error SVGs
  returned with exit 0: `Parser.getScale` calls `Double.parseDouble` before
  sprite lookup. They remain nonpositive, not repaired diagrams. Rust was not
  made to panic and negative acceptance/serialization was not changed.
- The historical welcome factory, class nested quotes, Unicode/property/
  surrogate, negative diagnostics and sequence panic dependencies remain.
  No scope waiver is inferred from this packet's ownership boundaries.

## Reference provenance

Only the parent's existing Monkey Man runner executed fresh Java. No local
JVM, javac, SSH, new runner or child agent was used. The immutable original
request is `request.json`, manifest `manifest.json`, bundle
`input-bundle.json.gz`, under private cache
`/Users/marcelo/Library/Caches/rustuml-sprite-corrective-maker`.
Exact source paths/bytes/SHA256/base64 and 15 resources/memberships are retained.

Returned gzip SHA256 is
`fa8edd802079d5837ee1a88ddf016af1b6ec48beabcc965dfcf9dbf4855976df`.
The validator independently checks the original envelope, every source and
stream hash, resource membership, exact wrapper arguments, private cwd,
`GMT+11:00`, environment, pinned headless Java revision/JAR and derived Java
argv. The JAR is
`aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.

The wrapper invocation was
`/Users/marcelo/rustuml-reference/bin/reference --cwd /Users/marcelo/rustuml-reference/scratch/worker.BoUn49wT/new --timezone GMT+11:00 -- -pipe -tsvg`.
Java argv is derived from pinned wrapper construction, **not process-traced**.
No global historical font or Graphviz equivalence is claimed.

All 15 fresh outputs are valid XML diagrams with no timeout, welcome/error
page, exception or process failure. The intended-negative `absent_labels`
actually selects Java CLASS and is retained as a valid passing input. Original
checker syntax/welcome and malformed-scale nonpositives remain separately
classified; intent and exit 0 never determine the denominator.

The parent initially assumed the wrong bundle schema and raised `KeyError`
before upload or Java execution. It losslessly mapped original
`files[].base64` into relay `resources[].data`, embedding exact original
request/manifest bytes. `parent-upload-validation.json`,
`parent-relay-validation.json`, both envelopes, and raw transport preserve
that event and mapping. The original bundle is unchanged.

## Final gates and reproducibility

Gates ran once after stable production and registration of all 15 fresh pairs.
`gates.json` records exact argv, cwd, environment, exits and duration; archive
logs preserve full output. Jobs, Rayon and Rust test concurrency were capped
at 4. Before/after target directories are private. `build-provenance.json`
records source revisions/trees, source hashes, binary hashes and toolchain;
`focused-commands.json` records focused tests. No broad gate was repeated for
this evidence-only packaging.

- Library tests: layout 41, math 10, oracle 31 and parser 622 pass. Renderer
  778/780: the same plain-theme class-default and initial-blank source-identity
  failures remain, with two new passing shared tests. The two existing unit
  inputs' CLI SVG/YAML are byte-identical before/after (`unit-residue.json`).
- CLI pipe tests: 15 pass.
- Main **library-path diagnostics**, not actual CLI counts: product tier
  11,179/11,251; assisted tier 10,497/11,251 eligible (12,550 total, 1,299
  skipped). Both exit 101. Existing activity/edge-case ratchet failures remain.
- Standing harness: 674/1,719; matching-environment actual CLI observations
  map to 704/1,719 (before 693/1,719 on the same membership). All pairs map;
  the same 30 include/environment discrepancies are explicitly listed.
- Guards: 4/6; decimal and review-record gates remain red. New accounts await
  an independent checker; this maker does not manufacture an attestation.
- Clippy exits 101 on unchanged `parse/component.rs:72` dead code.
- Workspace fmt remains red in unchanged parser/class and renderer/class.
  All three touched files' fmt and `git diff --check` pass.

`measurements.tar.gz` contains all private-cache evidence except rebuildable
binaries/targets and bytecode caches; `archive-files.json` hashes every member
and `archive-integrity.json` records verified archive membership and bytes.
It includes exact frozen cohorts, all CLI/YAML/comparator streams, failures,
original and mapped relay envelopes, raw returned provenance, fresh references,
logs and scripts. Historical resource captures are also retained in the
unchanged earlier maker/checker archives; their paths are not rewritten.
Fresh original `.puml`/Java `.svg` pairs are registered under
`test-diagrams/perturbations/creole-unresolved-sprite-command/corrective-maker/`.

Run the scripts from this checkout with `uv run --no-project --with pyyaml
python <script>` when Python YAML support is needed. `run_replay.py before`
and `run_replay.py after` replay the four immutable original memberships;
append `fresh15` for the fresh membership. `validate_fresh.py` independently
validates/classifies the returned raw capture without executing Java.
