# Independent unresolved-sprite review — rejected

The exact candidate `246ff6396b538843143ee9d2cc9a569b67f614dd` is **rejected**.
The shared command-consumption model has bounded support, but 12 valid fresh
whole-SVG counterexamples and required gate failures remain. No scope waiver
is granted. No production code, historical evidence, baseline, comparator,
skip, allow-list, target or shipping state was changed.

The checker was a separate, nonimplementing Codex/T3 task using
`gpt-6-astra/high`, the same model family as the maker. It used the prepared
checkout `/Users/marcelo/.t3/worktrees/rustuml/t3-9a8dea60`, branch
`review/unresolved-sprite-246ff639-astra-20261104`, without child agents or
another worktree. Only this checkout was temporarily switched to the base to
build the before CLI, then restored to the review branch at candidate.

## Actual CLI results

| Immutable membership | Entries | Valid Java references | Base passes | Candidate passes |
| --- | ---: | ---: | ---: | ---: |
| Retained TIM cohorts, including checker8 | 1,730 | 1,609 | 709 | 709 |
| Maker matrix | 18 | 17 | 3 | 13 |
| Existing golden sprites | 50 | 49 | 49 | 49 |
| Fresh checker matrix | 38 | 34 | 8 | 22 |

These are original-source `rustuml -pipe` executions and unchanged strict
SVG comparisons. CLI YAML projections are separately preserved. There are
no lost passes or changed models in these measured cohorts. The retained
membership has 1,651 distinct source/reference pairs and 79 duplicate entries;
the same 32 retained panic entries remain. Only `markup_literal` and
`markup_scan` change retained SVG bytes, and **both still fail**.

Fresh classification is independent of process success and original intent:
34 valid diagrams, two `NumberFormatException` crash SVGs returned with exit 0,
one empty-description welcome/error SVG and one syntax-error welcome SVG,
both returned with exit 200. The nested-start WBS control is observed valid.
There were no timeouts or zero-byte Java SVG streams. All stderr streams,
including empty ones, and all positive/nonpositive SVGs are preserved.
`fresh-classification.json` refines the initial mechanical error-page category
without rerunning either product or changing any raw output.

## Causal findings

Java `CommandCreoleSprite`, `Splitter`, `SpriteUtils`, and `StripeSimple`
support consuming a recognized unresolved command, flushing neighboring text
atoms, and inserting a base-font space if recursive style restoration leaves
an empty stripe. The Rust barriers preserve boundaries without changing the
public text APIs. Fresh command position, repetition, whitespace, optional
braces, case, Unicode names, and closed/EOL styles support that bounded model.
Malformed names, escaped markup and negative-scale literals retain passes.
No fixture-shaped branch, golden replay, coordinate tuning or unexplained
layout constant appears in the implementation diff.

The remaining failures have specific mechanisms:

- **Scanner recognition:** `nested_angle` hides a valid inner `<$south>` from
  Rust's first-`>` tag collector. Java scans unrecognized characters singly and
  reaches that command. Java width is 187; Rust width is 256.
- **Empty stripes:** `empty_lines` correctly consumes the sprite-only lines,
  but the ordinary blank line still emits empty text of length zero instead
  of Java's space atom. The note-specific sprite resolver also omits its
  sprite-only line in `note_undefined`; that control is byte-identical before
  and after.
- **Class placement:** `style_nested` matches text runs, style, width and x,
  but class header baselines still use base-font line metrics. The maker's
  `missing_size` failure remains. Shared command handling does not repair
  this caller's positioned-stripe contract.
- **WBS measurement:** both WBS controls consume tokens during emission but
  measure raw label text in `wbs::measure`. Java width is 78; Rust width is
  137. The title control passes. A diagnostic on its exact label separately
  demonstrates that `to_svg_tspans` still emits the sprite token literally;
  that API probe is not a whole-product score.
- **URL labels:** `url_label` and `empty_link` bypass shared recursive command
  parsing, while Java `AtomTextUtils.createAtomTextForUrl` handles sprite
  atoms. `url_after` retains the existing URL/italic parsing disagreement.
- **Defined consumers:** basic defined note and deployment controls pass.
  Defined class and sequence inline sprites remain unsupported; these are
  defined sprites, not absent ones. The defined note scale/color case fails
  because the existing family resolver uses the suffix as part of the lookup
  key. Those consumers retain their own registry ownership.
- **Numeric side effects:** `.` and `1..2` scales match the Java command
  grammar but throw in `Parser.getScale` before lookup. Rust consumes them as
  ordinary missing commands. Their crash SVGs are retained as nonpositive
  references, never counted as repaired diagrams.

The original markup controls also require quoted-display generic extraction.
The separately unintegrated class candidate
`68ab512be964e243ae1943f7a9fefd3537091401` already splits the quoted label at
`parse/class.rs:993`. It was inspected read-only. No duplicate repair or
combined correctness claim is made. Future integration must rerun the original
whole-SVG controls after independently accepted sibling changes are combined.

## Required gates on the exact candidate

All broad gates ran once, before checker evidence registration, with jobs 4.
Their exact argv, cwd, environment, duration and nonzero exits are in
`gates.json`; raw logs are in the archive.

- Library tests: layout 41, math 10, oracle 31 and parser 622 pass. Renderer
  has 776 passes and **two failures**, for plain-theme class defaults and
  initial-blank source identity. Both exact test-input CLI SVG/YAML controls
  are byte-identical before/after. This is attribution evidence, not a waiver.
- CLI pipe integration: 15 pass, exit 0.
- Main product library diagnostic: 11,179/11,251, exit 101.
- Assisted library diagnostic: 10,497/11,251 eligible, 1,299 skipped from
  12,550 total, exit 101. Neither library diagnostic is relabeled as a CLI
  headline count.
- Standing candidate corpus: 635/1,666, exit 101. Matching-environment CLI
  mapping: 665/1,666. The maker's 622/1,648 and 652/1,648 preceded registration
  of its 18 cases; 13 added passes explain the increase. All 30 environment
  and include-base discrepancies remain enumerated. This is registration
  growth, not a further repair.
- Guards: 4/6, exit 101: decimal ratchet and review-record completeness.
- Clippy: exit 101 at unchanged `parse/component.rs:72`, before full lint
  coverage. Workspace format: exit 1 in unchanged files. Touched-file format
  and whitespace check: exit 0.

The new 38-case standing-algorithm diagnostic passes 22/38. All 38 are now
registered as executable pairs. No completed broad suite was rerun merely
for evidence registration, and no green full-suite claim follows from it.

## Provenance and reproduction

`audit-summary.json` verifies the exact base → pre-edit account → implementation
→ candidate chain. The implementation cites account
`b160f41e2076b98805d4e3c9262a46030394e67c`; implementation is
`78f06098b74a81d24efcc922b3116d03ad496132`. All 9,731 historical files were
independently enumerated from the base and compared byte-for-byte, not merely
counted from the maker's list. The only production change is `creole.rs`;
`text_render.rs` adds a test. TIM, public APIs and protected harness files are
unchanged.

The initially empty golden submodule was hydrated from a local clone at exact
`9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a` before running gates; 12,550 sources
were verified. No missing-corpus result was presented as evidence. Private
before/after target caches were cloned from existing caches and rebuilt on the
exact revisions; `builds.json` names the frozen executable hashes. An initial
standalone diagnostic build failed with E0460 while the candidate's oracle
rlib was stale; its log is preserved. After candidate compilation completed,
the unchanged diagnostic sources compiled successfully. No product or test
failure was hidden by that setup retry.

Only the parent's established Monkey Man runner executed Java; this checker
ran no Java/javac, SSH or remote runner. Reference source is pinned at
`71806a23780b04a5ccde2f8ceb5121edad5eb711`; JAR SHA256 is
`aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
All 38 source/resource hashes and memberships, raw streams, statuses, wrapper
arguments, private cwd, timezone `GMT+11:00`, environment and JAR identity were
independently validated. Java argv is derived from the pinned wrapper's
construction, **not process-traced**. Historical binary/font/Graphviz and
universal Unicode-property equivalence remain unproven.

- Request SHA256: `689ab4642e561b4ca32b9fc6c6fb897e9726d1a10e7aa2da8c171cfea819f620`.
- Input bundle SHA256: `6b56b1939100abddff0608c128d92f6c1ad134a0f5c5f3f5a7254477f43c73e2`.
- Returned gzip SHA256: `d7ef84c5920c5ae2f9a17c77e90aeebc55e5aa6ce9ab6094ebdb6437f19c3908`.

`measurements.tar.gz` retains the complete private evidence tree except build
caches and executable binaries. Restore under
`/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-checker`; its hash and
exclusions are in `archive-integrity.json`. `capture-paths.json` maps external
replay sources, references and include resources to bytes in `capture-bytes/`;
restore these paths or explicitly remap them without normalizing source.
Repo-owned resources come from the exact candidate and frozen corpus. Rebuild
before/after CLI binaries with the recorded jobs-4 release commands, rebuild
the unchanged comparator and probes with `diagnostic-builds.json`, then use `replay-frozen.py <before|after> <retained|maker18|sprites50>
<new-private-output-directory>` to replay the frozen memberships. The scripts and raw result records disclose all
reused maker measurement helpers; independent source analysis, classification,
fresh cases, comparison execution and this verdict belong to the checker.
