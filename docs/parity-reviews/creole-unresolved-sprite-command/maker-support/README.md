# Shared unresolved sprite command maker packet — checker pending

This is a new implementation packet, not a review or acceptance of TIM8. The
maker worked alone through Codex/T3 as gpt-6-astra/high in the prepared checkout
`/Users/marcelo/.t3/worktrees/rustuml/t3-a295df45`, branch
`impl/unresolved-sprite-tim8-20261104`. No children, local Java/javac, own SSH,
target edits, parser/family renderer edits, push, PR or release occurred.

- Exact base/rejected TIM8 evidence: `0f00130a26b2f300996d0bd427f9e70b4ef10e30`.
- Pre-edit causal account: `b160f41e2076b98805d4e3c9262a46030394e67c`.
- Implementation: `78f06098b74a81d24efcc922b3116d03ad496132`.
- Production ownership: `creole.rs`; `text_render.rs` adds one focused test.
  Existing public Segment, Style and TextBase APIs are unchanged.
- All 9,731 prior account/review/evidence/perturbation files remain unchanged.
  No checker attestation is authored. The missing new review correctly keeps
  the evidence guard red. Every prior rejection and failing pair remains.

## Causal model and limits

At pinned Java `71806a23780b04a5ccde2f8ceb5121edad5eb711`,
`CommandCreoleSprite` recognizes `Splitter.spritePattern`, including the
`SpriteUtils.SPRITE_NAME` grammar and optional color/scale syntax. It consumes
the recognized command. `StripeSimple.modifyStripe` first flushes pending text;
`addSprite` adds an AtomSprite only when `SkinParam.getSprite` resolves a sprite.
A missing sprite produces no literal text and no dimensions, but the flush
still separates neighboring atoms. `getAtoms` supplies one space when the
stripe has no atoms. Size/color recursive commands restore the surrounding
font even in their EOL form before this fallback. `AtomText` and `SheetBlock1`
measure and position the resulting atoms, rather than measuring the original
markup. `../java-source-provenance.json` verifies the inspected source bytes.

Rust's shared segment parser previously emitted these tokens through the
unknown-HTML literal arm. Deleting tokens alone would incorrectly coalesce
same-style runs. The fix recognizes the Java grammar, uses empty segments as
private command barriers, then removes barriers from the final projection.
If consumption leaves the entire stripe empty, it supplies the space atom.
Measurement and emission already consume this same segment projection.
No coordinates, font metrics, asset data, branch-count rules or SVG surgery
were added. TIM is untouched; literal replacement values are not rescanned.

Existing sprite-aware note/deployment consumers resolve their images before
calling shared text rendering and retain their ownership. Shared text APIs
have no sprite registry. Defined inline class sprites were already unsupported
and remain an explicit failed consumer control; this change does not claim to
implement their resolution. The legacy tspan parser and sprite-in-URL label
parser are separate existing paths, not certified by this packet. Malformed
numeric scale handling and universal Unicode-property/JVM-version equivalence
are not certified by grammar recognition. These limits are not scope waivers
or acceptance of failing inputs.

## Actual CLI evidence

The CLI receives the exact original source bytes via `-pipe`; models use
`--yaml -pipe`. Java SVG comparison uses the unchanged strict comparator.
Direct evaluated-source probing is separate from both CLI model projection
and SVG truth. All raw streams, statuses, commands and source/binary hashes
are preserved in `measurements.tar.gz` and the small JSON records here.

| Frozen cohort | Entries | Valid references | Before CLI SVG | After CLI SVG |
| --- | ---: | ---: | ---: | ---: |
| Retained TIM cohorts including checker8 | 1,730 | 1,609 | 709 | 709 |
| Fresh source-guided maker matrix | 18 | 17 | 3 | 13 |
| Existing golden sprite family | 50 | 49 | 49 | 49 |

The retained cohort has 1,651 distinct source/reference pairs and 79 duplicate
entries; `summary.json` breaks it down by original membership. There are no
retained lost passes, new model changes or new panic cases. The same 32 sequence
panic entries remain. Fresh matrix membership is independent and immutable.
All 18 fresh direct sources and CLI-projected sources agree with their Java
source probes, including the syntax negative's diagnostic source. This does
not erase the retained negative-source mismatches or Unicode IntVal failures.
The existing sprite family has one preserved nonpositive reference, never
counted as a positive pass.

Ten newly matching CLI cases exercise leading/middle/repeated missing names,
spaces, bold, multiple lines, sequence messages, empty styled/unstyled stripes,
and name/color/scale command grammar. Existing valid note, defined-note and
invalid-marker literal controls retain their passes. The four valid fresh
whole-output failures are preserved:

- `missing_size`: runs, widths and x positions match; class header baselines
  still differ. The base caller computes name-line height/ascent from the base
  font at `class.rs:8456-8524`. The active class maker's separate positioned
  stripe API is not imported or certified here.
- `escape_url`: the pre-existing URL/italic parsing interaction differs from
  Java. This control fails both before and after; the sprite escape stays
  literal. No URL rewriting is introduced.
- `defined_scale_color`: the existing family-specific sprite resolver does
  not implement all defined sprite color/scale forms. It remains a failure.
- `defined_class`: the class label consumer lacks defined inline sprite
  resolution. The image remains absent; the whole SVG remains a failure.

`invalid_source` is an actual Java syntax negative, not a positive XML result.
Its source, error SVG, stdout/stderr and exits remain in the standing corpus.
The fresh standing-algorithm diagnostic passes 13/18 and preserves all five
failures. It uses the existing parser/render/comparator algorithm with no
layout injection or include base; it is not relabeled as CLI evidence.

Both original `markup_literal` and `markup_scan` still fail whole SVG on this
TIM8 base. `../dependency-handoff.json` names their exact original source and
reference paths, source hashes, Java parser provenance, and the tested Rust
revision. Java extracts the quoted display's trailing generic through
`NameAndCodeParser.DISPLAY_WITH_GENERIC` and `CommandCreateClass.executeArg`
before Creole rendering. This base misses that extraction. The separate
unintegrated class candidate `68ab512be964e243ae1943f7a9fefd3537091401` already
has `split_generic(&mut id).or_else(|| split_generic(&mut label))` at line 993.
That source was read only; no combined product run or acceptance is claimed.
Both pairs remain future combined-integration obligations. Source-line-zero,
welcome-factory, nested-quote class, Unicode/property/surrogate, negative
error-page and sequence panic dependencies also remain failures.

## Final gates and retained iterations

Required gates ran on the stable implementation. The first driver was
interrupted during compilation before any gate completed. Its partial logs
and `interruption.json` are preserved; the driver then resumed. No completed
broad gate was rerun for evidence-only registration.

- Library tests: layout 41, math 10, oracle 31 and parser 622 pass. Renderer
  has 776 passes and two failures: plain-theme class defaults and initial-blank
  source identity. Both test inputs produce byte-identical before/after CLI
  SVG and YAML (`unit-residue.json`); their code/assertions are unchanged.
- CLI pipe integration: 15 pass, exit 0.
- Main product **library diagnostic**: 11,179/11,251; exit 101.
- Assisted **library diagnostic**: 10,497/11,251 eligible, with 1,299 skipped
  out of 12,550; exit 101. These are not actual CLI headline counts.
- Standing corpus at gate time: 622/1,648; exit 101. Its actual CLI mapping
  yields 652/1,648 under the recorded source environments. The increase from
  prior reported 609/1,628 and 636/1,628 is TIM8's 20 cases registered after
  its own broad gate (13 harness and 16 CLI passes), not this implementation.
  `standing-mapping.json` enumerates the 30 include/environment discrepancies.
- Guards: 4/6, exit 101 (decimal ratchet and review-record completeness).
- Clippy workspace/all-targets: exit 101 on unchanged
  `parse/component.rs:72` dead code, before complete downstream lint coverage.
- Workspace formatting: exit 1 in unchanged parser/class and renderer/class
  code. Touched-file rustfmt and implementation whitespace: exit 0.
- Focused final sprite tests: 19 pass. The first focused run passed 18/19;
  the new test expected raw NBSP although the established emitter uses
  `&#160;`. Only that test expectation changed; the failed log is retained.

`gates.json` records exact argv, cwd, environment, duration and exit status.
The fresh text-log adapter printed two redundant labels for its single phase;
the logs are retained verbatim. The phase-keyed fresh-before/fresh-after JSON
records and `summary.json` are the correct comparison, without re-running or
rewriting measured outputs.

## Reference process and reproduction

Only the parent's existing Monkey Man runner executed fresh Java. All 19
resources, 18 memberships and 36 SVG/unchanged SourceProbe calls were
independently validated, including exact bytes, hashes, stdin, actual wrapper
and Java argv, cwd, timezone, locale, encoding and headless flags. There were
no timeouts. The wrapper is `/Users/marcelo/rustuml-reference/bin/reference`
with `--cwd <worker>/new --timezone GMT+11:00 -- -pipe -tsvg`. SourceProbe uses
the existing TIM schema and `<worker>` placeholder unchanged.

- Request SHA256: `7c429ca1334753052b3eeb73b3c707ac76756ff821d0d1786e61ba4ee793821e`.
- Input bundle SHA256: `b3c32f5b8dc8d6a4b9adc7ec564795058833465e9792c2e5d5cd5f3ab4a50d49`.
- Returned gzip SHA256: `ee508b14b86386b5c3d52edeafb3f50475c5bd4815f523dfa67a6299a907ca2c`.
- Pinned JAR SHA256: `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.

This proves the captured process contracts, not global historical Java/font/
Graphviz equivalence. Raw transport and both parent and maker validation stay
in the archive; decoded streams and commands are also directly under `cases/`.
All 18 exact source/Java SVG pairs are registered under
`test-diagrams/perturbations/creole-unresolved-sprite-command/maker/`.

Restore the archive to `/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-maker`
for the recorded paths. `retained-input-paths.json` maps each externally
captured source/reference/resource path to exact bytes under
`retained-input-bytes/<sha256>`; restore that hierarchy or explicitly remap
paths, never normalize source bytes. Repo-owned include resources stay in the
unchanged historical tree. The golden gitlink remains
`9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`.

Build the base and implementation into separate private targets with
`CARGO_BUILD_JOBS=4 cargo build -p rustuml --release`, copy each frozen binary
as `before-rustuml`/`after-rustuml`, and rebuild the unchanged comparator/source
probe as documented in `diagnostic-provenance.json`. The initial focused unit
compile reused the before target *after* its CLI executable was frozen; after
CLI construction and final gates use the separate after target. Binary and
source hashes in the two build records identify exactly what was measured.

Run `replay.py before` / `replay.py after` for the original retained membership,
`fresh-before.py` / `fresh-after.py` for the immutable fresh reference validation
and CLI observations, and `focused-collateral.py` for the sprite-family and
unchanged unit-input diagnostics. `checker8-membership.json` uses the same
`replay.measure` operation. `fresh-standing.py` invokes the unchanged standing
algorithm only on the fresh matrix. These are maker measurement scripts, not
independent checker attestations. A new independent checker must attempt to
disprove the causal model and create fresh held-out cases before acceptance.
