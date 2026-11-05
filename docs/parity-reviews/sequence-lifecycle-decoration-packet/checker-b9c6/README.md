# Independent sequence packet review

**Two bounded acceptances; four rejections. No whole-sequence acceptance.**
The candidate's reported gains reproduce, but fresh source-guided counterexamples
disprove four mechanism claims. Shared dependencies remain part of those claims.
No earlier review or counterexample is replaced or waived.

Reviewer: gpt-6-astra through the Codex harness in T3 Code, independent task
`thread:delegated-task:command%3Amcp%3A7a6e5142-c330-4dbe-941b-87119bcc7cef%3Adelegate-task%3A185d7cbb-70b4-4f24-9abb-f542386743b4`.
Worked alone in the prepared checkout on `review/sequence-packet-b9c6cf04`.

- Tested candidate: `5358314a0180cf760584c79bb5efd4ec1ae6320b`.
- Production revision: `41e2ce88aec0c6856d57f0c2322ca097ecd55de4`.
- Before: `0fb4e14b7227eb768d268128d42fa5f6d5a5b1af`.
- Java source: `71806a23780b04a5ccde2f8ceb5121edad5eb711`.
- Remote jar SHA256: `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.

| Mechanism and independent record | Verdict | Fresh valid matches / valid mismatches / invalid refs |
| --- | --- | --- |
| [Command event identity](../../sequence-command-event-identity/review.json) | Rejected | 2 / 8 / 4 |
| [Teoz life-event scheduling](../../sequence-teoz-life-event-scheduling/review.json) | Rejected | 3 / 3 / 0 |
| [Self-cross decoration](../../sequence-self-cross-decoration/review.json) | Accepted, bounded | 8 / 0 / 0 |
| [Legacy destroy component](../../sequence-legacy-destroy-component/review.json) | Accepted, bounded | 4 / 0 / 0 |
| [Legacy message-note envelope](../../sequence-legacy-message-note-envelope/review.json) | Rejected | 5 / 1 / 0 |
| [Legacy message-note alignment](../../sequence-legacy-message-note-alignment/review.json) | Rejected | 1 / 10 / 0 |

The two acceptances concern the self CROSSX endpoint/two-stroke equations and
legacy destroy ordinate/preferred-height allocation at this complete candidate.
They do not certify arbitrary sequence styles, unrelated lifecycle behavior, or
an extracted/recomposed subset of commits. Default destroy styling remains a
shared limitation. The rejected mechanisms cannot be accepted by dropping their
downstream color, note Display, activation style, or parallel-layout dependencies.

## Causal findings and next repair boundaries

[source-findings.json](source-findings.json) gives exact Rust/Java locations,
case names, causal diagnoses and bounded next repair guidance. All eight findings
are pre-existing failures, not new lost passes.

- **B9-COLOR:** the 157-name/symbol and hex-length validator matches Java, but
  returns carry unnormalized strings into a narrower SVG color resolver.
  Transparent, single-nibble gray and alpha colors fail in both engines.
  Preserve color semantics through configuration and paint; do not reject valid colors.
- **B9-LEGACY-LIFE:** command return ownership is correctly separate from paint
  state. The legacy painter discards an unmatched close, while Java's raw
  variation count affects `finish/getSegment`; Rust draws an extra activation.
  Repair the legacy variation stream, not the return stack.
- **B9-PARALLEL-CONTACT:** `TileParallel` aligns component contact points, which
  differ from drawing origins and lifecycle callback points. Rust conflates them;
  self/ordinary parallel controls put ordinary strokes and activations 2.5 units
  too high. Represent the distinct ordinates rather than adding an offset.
- **B9-PARALLEL-PREDECESSOR:** Java can capture a standalone note as the preceding
  graphical parallel tile; Rust resets its message list and serializes the next
  parallel message. Compose tiles plus trailing life events, preserving the
  separate stairs-reset rule.
- **B9-DESTROY-ORDER:** Java paints each level's bar and closing cross before the
  next level. Rust paints all bars before all crosses. Preserve an ordered paint
  stream; do not sort SVG output.
- **B9-NOTE-DISPLAY:** Java expands escaped newlines before note measurement;
  Rust measures the inline text as one line. A block-form control passes.
  Normalize Display semantics before shared measurement and painting.
- **B9-NOTE-ALIGNMENT:** explicit `noteTextAlignment` never reaches the new margin
  arithmetic; scoped numeric note shadow never reaches component dimensions or
  paint. Feed effective note style into both paths, without double shadow.
- **B9-THEME-ACTIVATION:** the single-line cerulean note envelope matches, but
  activation bars retain a hardcoded stroke instead of the merged Java style.
  Carry the activation style through materialization and painting.

The seven supplemental inputs discriminate escaped versus physical multiline
notes, explicit CENTER/RIGHT versus default LEFT, global versus scoped numeric
shadow, and note bounds versus themed activation paint. The first 42 inputs
cover engine, parallel ancestry/return ownership, exo endpoints, creation and
destruction depth/order, valid/invalid colors and transitions, self/reverse/cross
ends, and ordinary note side/alignment/style/activation. Every case has at least
two relevant axes in its record. All 49 source/Java pairs, before/candidate CLI
outputs, stderr and per-case provenance are retained under the six mechanism
directories in `test-diagrams/perturbations/`.

## Source and chronology audit

[audit.json](audit.json), [source-audit.json](source-audit.json) and
[protected-audit.json](protected-audit.json) preserve the account ancestry,
production hashes, named dataflow, constant/branch provenance and limitations.
All six accounts and three source refinements precede their relevant production
changes. The four production files have the claimed hashes; the candidate and
production revisions have identical crate contents.

The source correction is explicit: **`x--` is forward starting CROSSX,
`reverseDefine=false`; `<-x` is the reverse branch.** The initial wrong account
hypothesis remains immutable and is corrected by pre-edit refinement `42ec7126`.
The production parser was not changed to make `x--` reverse.

Exactly two `ascii.rs` test constructors gained `parallel:false`. No ASCII
behavior changed. The independent serde probe confirms both `Message.parallel`
and `ReturnMessage.parallel` deserialize missing fields to false and preserve
true on roundtrip. Same-message forbidden reactivation and unknown return
colors reject; valid other-participant transitions remain distinct.

No new fixture-label branch, golden read in shipping code, branch-count ladder,
coordinate-keyed SVG surgery, or high-precision metrics table entry was found.
Generic created-head group unwrapping was moved from existing code. The source
audit explicitly limits the inherited two-unit coordinate convention and fixed
destroy style; reuse is not proof of general parity.

## Independent replay and test results

The unchanged replay helper and comparator adapters were run with independently
built source-only CLI binaries. Before source was obtained by `git archive` into
a private cache, with separate before/after Cargo targets and binary directories.
No other worktree was created or changed. Replays use `rustuml -tsvg -pipe`;
library product/assisted measurements are separate diagnostics.

The exact 1,828-entry replay reproduced 1,208 matches / 581 valid mismatches /
39 invalid references before, and 1,232 / 557 / 39 after: 24 gains, no lost
passes, no new valid CLI errors, CLI panics or timeouts. The 19 corrected negative
rejections are recorded separately and are not positive parity passes. All
source/reference hashes and both before/after output hashes equal the durable
maker captures. The 48 duplicate-source-byte entries are retained: 1,780 distinct
byte strings and 1,828 paths. Frozen955 remains 403 matches / 542 mismatches /
10 invalid references. All 1,789 valid CLI outputs agree with product-library
results; assisted diagnostics give 1,189 matches / 600 valid mismatches.

Fresh 49: **23 valid matches, 22 valid mismatches, four invalid Java references**;
the four negatives are correctly rejected by Rust. There are 19 fresh gains,
no lost passes and no valid CLI errors or panics. Invalid Java references never
count as passes, including when recording successful negative rejection.

| Check | Result / exit |
| --- | --- |
| Parser and renderer library units | 567 + 783 pass / 0 |
| Real CLI pipe tests | 13 pass / 0 |
| Candidate and before release builds; parser/render check | Pass / 0 |
| Changed four Rust files, `rustfmt --check` | Pass / 0 |
| Main product **library** tier | 11,181 / 11,251; sequence 1,411 / 1,411; exit 101 |
| Main assisted **library** tier | 10,497 / 11,251; exit 101 |
| Guards on populated corpus | 4 / 6; exit 101 |
| Standing perturbations with all 49 fresh inputs | 637 / 1,240; 603 fail; exit 101 |
| Review-record guard after new records | Exit 101, expected historical and new rejected/failing records remain enforced |

The initial main-tier invocation failed setup because this prepared checkout's
golden submodule directory was empty; it rendered no main-corpus inputs and its
guard result was 3/6. That log is retained. The exact gitlink
`9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a` was then populated from the locally
available archive, without editing reference bytes. Each main tier ran once
against that populated corpus. Product's stale improvement ratchets and
assisted's 754 retained failures make both tests nonzero. Their failing name
sets exactly match the maker's saved sets. The two populated-corpus red guards
are `decimal_literal_ratchet` and `parity_review_records_are_complete`.

The unchanged positive perturbation suite counts the four fresh invalid Java
references among its failures. It remains an enforcement point, not a waiver.
The post-record guard reports only expected rejected/failing-verdict complaints
for these six records; schema, paths, axes and hashes were independently checked.

All relevant nonzero exits are preserved. In replay library diagnostics, 38
invalid references return 2 and the pre-existing empty invalid-date reference
causes one `diff_one` adapter panic/exit 101; that is not a CLI panic. Remote
Java's four semantic-error cases return 200. Python orchestration may exit zero
after recording child failures; the child statuses in JSON/logs are authoritative.

Cargo build jobs were capped at four and `RAYON_NUM_THREADS=4` was set. The
unchanged main-tier harness explicitly configures its own pool from `num_cpus`;
this evidence does not claim its total runtime thread count was capped at four.
Private persistent cache: `/Users/marcelo/Library/Caches/rustuml-sequence-review-b9c6cf04`.

## Remote reference provenance and retained residue

No local Java, javac, SSH retry or remote agent was used. The parent relayed the
exact immutable manifests and bytes to its existing Monkey Man runner using
`/Users/marcelo/rustuml-reference/bin/reference --cwd PRIVATE_CWD --timezone GMT+11:00 -- -pipe -tsvg`.
Each raw capture includes the concrete private cwd, exact wrapper and headless
Java argv, jar identity, source membership/hash, stdout/stderr bytes and hashes,
exit status and timeout flag. The checker independently revalidated all of these.

- Batch42 manifest SHA256: `3c82996facda38530fcbedb3e036c53eaeea31d4f7ac2d539f232f435736260b`.
  Raw gzip SHA256: `55564eb56482001915e1c72b58b46373d2dea1b3be61853a640ef0eef7566081`.
  Worker `/Users/marcelo/rustuml-reference/scratch/worker.NDRkqqE9`;
  38 exit-zero and four exit-200 captures, no timeouts.
- Supplement7 manifest SHA256: `bab660c19142967540728b64fb8c00cd1da33d63d6d120895a54f8d08cb403d3`.
  Raw gzip SHA256: `482551b1dbac55b80a242e697981c24136709b36130084335c2cc44975faac13`.
  Worker `/Users/marcelo/rustuml-reference/scratch/worker.BnjW88Pa`;
  seven exit-zero captures, no timeouts.

Remote Java 21.0.5 / Graphviz 12.2.0 differ from the historical environment. This
source-pinned jar is not the historical binary; font and Graphviz equivalence
are not universal. Historical GET references remain unchanged. Raw pipe/file
reader policy and explicit private cwd were retained; no frontend normalization
was substituted. No cases have external resources requiring transfer.

The fifteen named deferred checker mismatches, prior 542 frozen mismatches,
remaining twelve invalid-input acceptances, and shared fonts/styles/entities/cwd
dependencies remain active. The earlier 1,449-entry parent inventory overlaps
1,123 paths with maker1728; its 326 parent-only cases were not rerun by maker or
this review. This review does not imply parity for those unmeasured entries.

Raw remote JSON is retained losslessly compressed alongside transport metadata,
manifests and validation records. `replay-cli-captures.tar.gz` retains all 1,828
before/after CLI stdout streams; compressed measurements retain CLI stderr,
comparator and library diagnostics. Logs, exact command/environment records, source-guided generators,
import validators and packaging scripts are adjacent. Scripts intentionally name
the original private paths for provenance; rerunning assembly over an existing
review is not intended to overwrite its verdicts. `evidence-validation.json`
records the final pair/schema/hash checks. No production, baseline, comparator,
skip rule, historical reference/review or target file was edited.
