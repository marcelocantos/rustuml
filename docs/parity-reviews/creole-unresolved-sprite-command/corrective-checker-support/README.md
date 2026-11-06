# Corrective sprite packet: independent review rejected

Reviewed exact packet `a84856637adcd8e8d4e78fd2ee00658c9c612feb`, production
`59dc60604792260c342f6e6ab96829f10b0a59f5`, against clean
`246ff6396b538843143ee9d2cc9a569b67f614dd`. This separate gpt-6-astra/Codex/T3
review task authored three new rejected `review.json` records. No production,
account, comparator, baseline, skip, allowlist, target or shipping changes were
made. The maker and reviewer use the same model family; independence is by
task and authorship, not a claimed cross-model check.

## Decision and counterexamples

The correction has reproducible bounded gains, but six fresh valid whole-SVG
counterexamples reject acceptance under the existing rule. All inputs and
same-stem Java SVGs are registered under the three mechanism directories.
None is a newly lost pass: each failed before and after. They remain obligations,
including failures in pre-existing dependencies; ownership is not a waiver.

- **Candidate traversal:** `empty_bold_recognition` disproves equivalent
  recognition. Java `CommandCreoleStyle.matchingSize` returns captured body
  length. The closed empty `<b></b>` candidate returns zero; the next EOL
  candidate consumes `<b>` and paints the closing tag in bold. Rust recognizes
  `b` by name and erases the empty bounded span. Java paints `A<outer`,
  `</b>>B`, `C`; Rust paints `A<outer>B`, `C`. Widths: Java 217, Rust before
  243, Rust after 179. Fresh mechanism cohort: 7/8 valid pass after, 1/8 before.
- **Present empty stripe:** `present_bold_empty` is falsely classified as
  atomless upstream. Java paints `</b>`, `</i>`, and size-26 `</b>`; Rust emits
  three size-13 spaces. Java 173×169; Rust 145×153. Size-command empty bodies
  follow a different recognition rule and correctly project a restored space.
  Fresh cohort: 7/8 after, 4/8 before. Actual blanks at beginning/middle/end,
  size/color recursion, base size 7, and absent class/state/activity/sequence
  labels pass. The private diagnostic confirms lexical `parse_segments("")`
  remains empty while actual-line width, paint, height/ascent, first baseline,
  and element/baseline counts agree on the projected space.
- **WBS semantic text:** `wbs_multiline_root`, `wbs_multiline_child`, and
  `wbs_newline_empty` show literal backslash-n painted as text instead of
  Java's `Display.getWithNewlines` and stacked SheetBlock stripes. Java/Rust
  after canvases: 152×163/152×149, 205×221/231×207, 94×205/139×149.
  `wbs_boxless` shows the `_` shape marker left in Rust's label and a blanket
  rectangle, whereas Java's `IdeaShape` selects an unboxed text block:
  181×187/222×205. Fresh WBS cohort: 5/9 after, 1/9 before. Single-line boxed
  mixed-style, mixed-baseline, empty-root/child and deep left/right controls
  pass. Padding 10 is supported by pinned `plantuml.skin`; no new unexplained
  geometry constant or changed topology formula was found.

Next implementation obligations are model-level command recognition (closed
versus EOL alternatives), then WBS display lines and shape ownership before
claiming SheetBlock equivalence. This review implements none of them and does
not authorize coordinate compensation or a feature expansion. Future corrected
packets require independent re-review retaining every failing input.

## Exact actual CLI measurements

Separate private release targets built the clean before and exact candidate
with jobs 4. Each row used the original-source binary with `-pipe`, separately
`--yaml -pipe`, and a privately compiled diagnostic calling the unchanged
`compare_svg_strict`. Duplicates and all nonpositive/error members remain.

| Frozen cohort | Entries | Valid | Before pass | After pass |
| --- | ---: | ---: | ---: | ---: |
| Retained TIM/checker8 | 1,730 | 1,609 | 709 | 709 |
| Original maker | 18 | 17 | 13 | 13 |
| Golden sprites | 50 | 49 | 49 | 49 |
| Rejected checker | 38 | 34 | 22 | 26 |
| Corrective maker | 15 | 15 | 6 | 13 |
| Fresh independent checker | 26 | 25 | 6 | 19 |

There are zero lost passes across all six memberships. Retained membership has
1,651 distinct pairs and 79 duplicates. SVG exits remain 1,605 exit-0, 93 exit-1
and 32 exit-101 panics; YAML exits remain 1,622 exit-0, 93 exit-1 and 15 exit-101.
Other cohorts have only CLI exit-0 outcomes. Every stderr is preserved, including
empty streams. Fresh Java has 26 exit-0 outcomes and zero timeouts; `malformed_dot`
is nevertheless a `NumberFormatException` error SVG, not a valid diagram. Its
schema `result: fail` is explicitly nonpositive and does not demand a Rust panic.

All full parsed YAML trees are semantically equal before/after. The one byte
change, `sprites/sprite_multiple`, is map serialization order. All 207 valid
retained and 17 valid original-maker Java evaluated-source comparisons are exact.
Checker38, corrective15 and fresh26 have no Java SourceProbe; no such comparison
is claimed. Raw YAML and original sources are retained, not only projections.

The four repaired checker inputs are exactly `nested_angle`, `empty_lines`,
`wbs`, `legacy_wbs`. All eight other valid checker failures remain:
`style_nested`, `url_label`, `url_after`, `empty_link`, `note_undefined`,
`note_defined_scale`, `class_defined`, `seq_defined`. Original malformed `.` and
`1..2` scale references remain exit-0 Java exception SVGs; welcome/syntax
nonpositives retain their original memberships. Corrective `absent_labels` is
observed valid CLASS despite original negative intent. Corrective
`nested_unknown` and `nested_unicode` remain failures.

`markup-controls.json` preserves both original `markup_literal` and `markup_scan`
sources, before/after model YAML, reference hashes, commands and failure results.
Their SVG and YAML bytes are unchanged. On this tested revision, class parsing
splits generics from a label only when derived from the id. Separate candidate
`68ab512be964e243ae1943f7a9fefd3537091401` already has quoted-label generic splitting
and positioned-stripe caller work. It is not integrated or tested in combination;
this report does not duplicate that obligation as absent on the sibling.

Pinned Java `CreoleParser:175` calls `Guillemet.manageGuillemet:76-84` before
StripeSimple. This tree's sequence `process_label:2231` omits that projection;
participant handling at 6730 has it. The corrective `nested_unknown` failure
therefore remains caller-owned, not a reason for global shared normalization.
`nested_unicode` retains the known font-width mismatch and unsupported
`FontStyle.PLAIN`. `note_undefined` is a family resolver failure; defined note
scale/color, class and sequence sprites need their registry consumers. URL/italic,
legacy tspan, source-line-zero, welcome, nested quoted class, Unicode/property/
surrogate and negative semantics remain failures with no scope waiver.

## Broad gates, run once

After the exact candidate was stable, one gate sequence ran with Cargo, Rayon
and Rust test concurrency capped at 4. Fresh pairs were registered before the
standing gate. No broad gate was repeated after evidence-only work.

- Layout 41, math 10, oracle 31, parser 622 and CLI pipe 15 pass.
- Renderer 778/780: the same plain-theme class-default and initial-blank
  source-identity failures. Both newly added shared tests pass. Separate
  actual-CLI captures of the two old unit inputs are byte-identical before/after.
- Main product **library-path diagnostic**: 11,179/11,251; assisted diagnostic:
  10,497/11,251 eligible (12,550 total, 1,299 skipped). Both exit 101. These are
  not CLI measurements. Existing activity/edge-case ratchet failures remain.
- Standing original membership remains 674/1,719, versus matching-environment
  actual CLI 704/1,719 (before 693). Fresh registration adds 19/26 harness and
  CLI passes (before CLI 6). Expanded totals are 693/1,745 harness and 723/1,745
  matching-environment CLI (before 699). All pairs map; the same 30 include/
  environment discrepancies remain. Registration growth is not another repair.
- Guards: 4/6; decimal ratchet and review records are red. The gate preceded
  authoring the three rejection records; those records do not turn it green.
- Clippy fails on unchanged `parse/component.rs:72` dead code. Workspace fmt
  fails in unchanged class files. All three touched files' fmt and diff check pass.

The exact golden submodule `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a` was initially
unhydrated, then hydrated successfully from the existing local clone. Its full
12,550 sources were present before the gates. Raw setup/build/gate logs remain.

## Provenance and preservation

All 9,731 inventoried historical files and all 118 files imported from rejection
`83469a4832cf20b8e2e57514841e5c46adf63a6e` retain exact bytes. The rejection import
`6b0ca33e88127a44bad1e564544db7d746816d6d` precedes account commit `ee026189` and
location correction `e25a4a20`, both preceding production `59dc6060`. The latter
references the account commits. Production changes only `creole.rs`,
`text_render.rs`, `wbs.rs`; public APIs are unchanged. No fixture-shaped branch,
new unexplained constant, SVG coordinate surgery or protected gate change found.

Only the parent's existing Monkey Man runner executed fresh Java. No local JVM,
javac, SSH, child, new runner or worktree was used. Java source is pinned to
`71806a23780b04a5ccde2f8ceb5121edad5eb711`, JAR SHA256
`aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
Read-only local Java source bytes were checked against that Git revision.

The immutable original bundle SHA256 is
`5a76331dd6f7f8759f19d0330b3a0eb2aa43142702fbb3fcd90311b1da5bc0c8`;
returned gzip is
`275dcf6c83a99246c947016edafc7f92e4d8b9c4c154961f69e7f106dea78c45`.
The lossless `files[].base64` to `resources[].data` relay mapping preserves exact
original request/manifest bytes. Independent validation checked every source,
resource, raw stream, status, timeout, cwd, environment, timezone and wrapper
argument. Actual wrapper cwd was
`/Users/marcelo/rustuml-reference/scratch/worker.zu5h6zuK/new`, timezone
`GMT+11:00`. Java argv is derived from pinned wrapper construction, **not
process-traced**. Historical font/Graphviz environment equivalence remains unproved.

`measurements.tar.gz` retains all private evidence except rebuildable binaries,
target directories and bytecode caches. `archive-files.json` hashes each member;
`archive-integrity.json` verifies membership and exact bytes. Earlier retained
resource captures remain in unchanged historical archives; original cwd/include
contracts are preserved. The archive contains frozen cohort memberships and
input bytes, every CLI SVG/YAML/comparator stream, fresh request/relay/returned
captures, original markup models, full observations, source audits, scripts and
logs. Rebuild commands and binary/source hashes are in `build-provenance.json`;
actual commands/cwd/environment/exits are in per-case results and `gates.json`.

Replays use `uv run --no-project --with pyyaml python run_replay.py before`
(or `after`) with explicit cohort names `retained maker18 sprites50 checker38
fresh15 fresh26`. Extract the archive to a private cache, rebuild the pinned
binaries and unchanged comparator there, and honor the recorded cwd/include
resources. `validate_fresh.py` validates the recorded Java capture without
executing Java. These assets preserve reproducibility; the executed results,
not their existence, are the evidence.
