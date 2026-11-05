# Independent recursive hiding and command-flush review

**Both mechanisms are rejected.** Fresh valid whole-output counterexamples remain.
The hide/restore operation and the empty-command barriers have substantial bounded
support; that support does not waive the whole-output acceptance rule.

Reviewer: GPT-6 Astra, high reasoning, Codex harness in T3 Code. This is a new
independent task with no prior maker/checker involvement. The T3-prepared clean
checkout was switched to `review/creole-hide-flush-independent-20261105` at
`2608469dace1f680a7e90c0895e44e657f8aaf7f`. No production, account, baseline,
comparator, skip, allow-list, target or old review was changed. No children,
additional worktrees, shipping, local JVM, javac, SSH or new runner were used.

The exact comparison is base `a3afe487a66ae26c73e2f1853d6c26fc2d228875`
(production `8df6bb53fac4fdf364a1a9aeea0b203cb6dab8e6`) against production
`a6e1718c4eb693aec99c3a107fd654afb8a4e4e3` at the supplied evidence tip.
Accounts `5d039c21c9374ee91904fe4da430c22ff6e2c4f3` precede the committed
production draft in ancestry. The import at
`f75103e509a5c5cd61341d78421a436f7103603f` is patch-byte-identical to rejection
`5ec48be56c4bce12ec646c29ed8e6d4be5e8518b`. Git cannot prove the absence of an
unrecorded earlier draft; archive timestamps would not establish that either.

## Actual CLI measurements

Both release CLIs were built independently from exact Git source snapshots into
initially empty private targets with jobs 4. The initial snapshots omitted the
vendored Graphviz directory and failed to build; both setup-failure logs and
statuses are preserved. Adding `vendor` from each exact revision resolved the
setup error. No source code or compiler flags were altered. The strict comparison
diagnostic was rebuilt from the unchanged oracle crate and probe source.

Every observation retains its original membership, duplicate observations and
nonpositives. Each source ran through `-pipe` and separately `--yaml -pipe`, with
its cwd/include/environment contract preserved through recorded private path
mappings. All raw models, SVGs, stderr, command statuses and comparisons are in
`measurements.tar.gz`. All 7,792 replay stdout/exit comparisons against the maker's
captures reproduce exactly.

| Cohort | Entries | Valid | Before CLI | After CLI |
| --- | ---: | ---: | ---: | ---: |
| Retained TIM/checker8 | 1730 | 1609 | 709 | 709 |
| Original maker | 18 | 17 | 13 | 13 |
| Existing sprite controls | 50 | 49 | 49 | 49 |
| Earlier rejected checker | 38 | 34 | 26 | 26 |
| Prior corrective maker | 15 | 15 | 13 | 13 |
| Prior checker26 | 26 | 25 | 24 | 24 |
| Prior style/WBS maker | 15 | 14 | 12 | 12 |
| Prior delimiter controls | 4 | 4 | 0 | 0 |
| Prior mid-line controls | 2 | 2 | 2 | 2 |
| Imported checker initial | 30 | 26 | 20 | 22 |
| Imported checker supplement | 8 | 7 | 1 | 7 |
| Maker compact12 | 12 | 12 | 1 | 11 |
| **Replay total** | **1948** | **1814** | **870** | **888** |
| **Fresh independent checker** | **28** | **28** | **2** | **21** |
| **Combined observations** | **1976** | **1842** | **872** | **909** |

The original nine cohorts remain **848/1898**; imported checker38 reproduces
**21 → 29/33 valid**. There are **zero lost passes, zero new process errors and
zero new panics**, for both replay and fresh sets. The same 32 retained SVG
panics remain. Retained SVG exits are 1605/93/32 for 0/1/101; YAML exits are
1622/93/15. Every other SVG/YAML invocation exits zero. Full YAML is byte-identical
for **1948/1948 replay**, **28/28 fresh**, **1976/1976 combined** observations.
Evaluated-source comparisons remain **207/207 retained** and **17/17 maker**.
These are separate from original-source CLI SVG comparisons.

## Fresh independent findings

The 28 fresh sources differ from every replay source and all 12,550 golden
source files (12,090 distinct source hashes). Each reference was independently
validated as a diagram using XML, factory, visible diagnostics, stderr and the
unchanged reference comparator; validity was not inferred from exit zero.
All 28 reference calls exited zero, with empty stderr and no timeouts.

| Mechanism | Whole-output result | Verdict |
| --- | ---: | --- |
| Recursive character hiding | 1 → 14/20 | Rejected |
| Parameter command flush | 1 → 7/8 | Rejected |

The new per-mechanism `review.json` files name every pair and its independent
axes. All 28 `.puml`/same-stem Java `.svg` pairs are registered under each
mechanism's `checker-independent-20261105/` perturbation directory. Every failed
case remains executable. The broad standing run predates these additions;
there is no post-evidence standing-green claim.

### Hiding and candidate selection

`CharHidder.hide`, `StripeSimple.analyzeAndAdd` and `AtomText` support the shared
hidden stream and literal-atom restoration. All five escaped overlapping
delimiter families, double/triple tildes, nested start/end escapes and the
activity/state/note/class-member consumers pass with exact text atoms. The
multiline WBS leaf/composed case also passes. No double-unescaping was observed
in these controls.

Two source-guided parameter cases disprove integrated candidate selection:

- `hide_parameter_size`: Java's `Splitter.fontSizePattern` requires digits.
  `X<size:~13>fir</size>Y`, a parameter containing backslash-tilde, and a trailing
  tilde parameter all render literally in Java. Rust's `starts_with("size:")`
  selects and consumes them despite the failed integer parse. Java has five
  total text atoms including participant labels; Rust has eleven.
- `hide_parameter_color`: `~#` becomes U+E023 before Java's candidate regex,
  which therefore does not match `#hex` or word color grammar. Java renders
  literal `X<color:#123456>fir</color>Y`. Rust selects by prefix and emits `fir`
  with **`fill="\uE023123456"`**. Hidden characters are restored for text,
  but permissive command selection passes them into style attributes. These
  cases failed before too; this is not reported as a newly lost pass.

`hide_closing_capture` has exact escaped-closing text after the correction but
fails its four-line folded-note border. `hide_raw_regions` retains false
`<code>` monospace activation and missing `<plain>` handling: the pinned Java
style registry recognizes `plain`, while Rust leaves it literal. Quoted
monospace escaping improves, but the whole case still fails. The supplementary
Unicode activity probe and isolated Omega control fail font/fallback widths,
not escape contents.

`hide_link_target` and `hide_link_label` pass whole output including URL and
tooltip attributes. Java itself carries hidden characters into some URL fields;
matching PUA attributes there are not a Rust-only leak. Link labels are raw
atom text, and the new control agrees on that boundary. This is bounded support,
not acceptance of the existing URL/italic/legacy-tspan obligations.

### Flush boundaries and the note counterexample

All eight flush cases have **byte-exact Java text atoms**, including counts,
content, style attributes, widths and baselines. Leading/trailing empty commands,
adjacent empties, nested consumed-empty bodies, nonempty siblings and multiline
WBS leaf/composed labels are supported. Barriers prevent equal-style coalescing
and are removed before drawing. An atomless actual stripe retains the base-style
space. No WBS origin or geometry was retuned.

`flush_eol_nested` nevertheless fails whole output: the folded-note bottom is
**121.4883 instead of 122.4883**, with all text atoms exact. Java's
`AbstractTextualComponent.getTextHeight` sums the text block and margins, then
`ComponentRoseNote.drawInternalU` casts the total height to `int`. Rust's
`note_rendered_height` floors each later line separately. For four size-13 rows:

- Java: `floor(4 × 15.310546875 + 10) = 71`.
- Rust: `floor(15.310546875 + 10) + 3 × floor(15.310546875) = 70`.

The same cause explains `hide_closing_capture`. This is an unchanged sequence
consumer failure, already present before. It rejects whole-output acceptance;
it does not justify altering the supported WBS leaf arithmetic or the barriers.

## Residual and neighbouring-control verification

The unchanged comparator emits exactly **one differing element attribute** for
each claimed maker residual; complete comparisons and per-element observations
are retained in `residuals.json` and the archive.

- `html_parameters`: only size-31 italic `</I>` baseline differs,
  **78.3672 versus 95.7695**. The missing ascent delta is
  `(31 − 13) × 0.966796875 = 17.40234375`. `emit_message_label` carries the old
  baseline into the new line, while shared emission interprets it as the first
  run's baseline. The corrected separate `q`, `r`, `s` atoms, styles, widths
  and all other comparator elements agree. Original size-26/25 failures remain.
- `literal_neighbors`: only `P~a~ΩQ` textLength differs,
  **48.0835 versus 51.2446**. U+03A9 is absent from the unchanged width table;
  `char_width` falls back to ASCII `a`. Replacing only Omega with `W` in the
  fresh control passes; the isolated Omega case reproduces the width defect
  and its downstream geometry. No text or style mismatch remains in the old
  maker case.
- `s_shape_color_literals` passes before and after. YAML independently confirms
  `_ Birch` and `_ Fir` are literal labels with omitted/default Box shape;
  adjacent `***_[#gold] Elm` is explicitly None. Its passing whitespace-neighbour
  recategorization is correct. It does not waive old-order warning failures.

These remain explicit failures: WBS old-order warning chrome (🎯T14.12) and
`wbs_shape_color`; the four sequence delimiter controls and headings/shared
stripe classification (🎯T14.13); mixed-size baselines; forced alignment;
unknown/guillemet, Unicode/font/PLAIN, note resolver, URL/italic/legacy tspan,
welcome and negative diagnostics. The nonpositive `malformed_dot`, two
shape-direction syntax-error SVGs, and three missing-Batik math references retain
their classifications and original bytes. No math parity claim is made (🎯T14.6.5).
The warning scope proposal remains a handoff, not an accepted implementation.

Unintegrated class candidate `68ab512be964e243ae1943f7a9fefd3537091401` and the
concurrent sequence work were not imported. No combined correctness is asserted.

## Once-only broad gates

The golden submodule was locally hydrated at exactly
`9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`; setup succeeded. Broad gates ran once
on the stable evidence tip before checker additions. Exact commands, timings,
environment and statuses are in `gates.json`; complete logs are archived.

- Layout 41, math unit 10, oracle 31, parser 624 and CLI pipe 15 passed.
- Renderer **784/786**, with the same plain-theme class and initial-blank
  source-identity failures. Their actual CLI SVG/YAML outputs are independently
  byte-identical before and after.
- Main product **11179/11251** and assisted **10497/11251 eligible**
  (12550 total, 1299 skipped) remain red **library diagnostics**, not CLI truth.
- Standing **752/1816**; matching-environment actual CLI **782/1816**, versus
  **764/1816 before**. The original 1766 members remain at 712 harness / 742 CLI
  passes. The 50 newly registered members contribute 40 harness passes and
  **22 → 40 CLI**. Registration growth is separate from 18 actual repairs.
  All paths map; the same 30 cwd/environment discrepancies remain.
- Guards **4/6**, with decimal and review-record guards red. Clippy remains red
  at `parse/component.rs:72`; workspace formatting remains red in unchanged
  class files. Touched-file formatting and diff checks pass.

## Provenance and preservation

Only `crates/rustuml-render/src/creole.rs` changes in production. WBS, public
text signatures, the legacy prefix including `to_svg_tspans`, `text_render.rs`,
class, sequence, TIM, fonts, metrics and geometry code are unchanged. Every new
parity-affecting constant/branch has the named Java provenance; no fixture-shaped
logic or unexplained new constant was found.

All **10,347 base artifacts**, the **9,731-file older inventory**, the **118**
earlier and **96** next rejection files, and all **135** imported rejection
files retain exact bytes. All **35,347 maker archive members** match their
inventory; archive SHA256 is
`c8d78419a7f297765ae725f4be2825f66cda1330855f7205c80509a4360e972f`.
All **32 maker-cited Java files** and **seven additional checker source checks**
match pinned `71806a23780b04a5ccde2f8ceb5121edad5eb711` (one additional check
rechecks FontStyle). Jar SHA256 is
`aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.

Only the parent's existing Monkey Man runner generated fresh references in
`/Users/marcelo/rustuml-reference/scratch/worker.0AbSTI3w/new`, GMT+11:00,
explicit env `{}`. The original immutable request/manifest/bundle, lossless
parent mapping, validation records and all returned streams are archived.
Returned gzip SHA256:
`58bcfa1c4a5dad74a5459edefc1e927ea43bbf273651892b8741bdf3080abbd7`.
Java argv is **derived from the pinned wrapper, not process-traced**.
Historical font/Graphviz environment equivalence remains unproved.

## Next obligations and reproduction

1. Enforce Java parameter candidate grammar before consuming size/color
   commands, including hidden characters; preserve the failing parameter pairs.
2. Preserve the now-supported barriers, recursive hiding and WBS arithmetic.
   Correct note aggregate-height rounding in its owning sequence work, then
   revalidate the unchanged whole failures; no offset tuning in WBS.
3. Retain the separate warning, stripe classifier, mixed-size, font, raw-style,
   URL and negative obligations. Restore a complete pinned reference environment
   before making any future math claim.

Extract this review's `measurements.tar.gz` into a private cache. `archive-files.json`
and `archive-integrity.json` verify every captured member. Source snapshots and
binaries are rebuildable from the exact revisions in `build-provenance.json`.
Rebuild the unchanged `probe.rs`, restore or losslessly map the captured cwd
resources, and run `run_replay.py` with the explicit cohort list recorded in the
review commands. `maker/frozen`, `maker/new` and `maker/fresh-reference` in the
archive preserve every replay input/reference; `new` and `fresh-reference`
preserve the new checker inputs/reference streams. The original maker packet
remains unchanged in Git. No reference validator executes Java locally.
