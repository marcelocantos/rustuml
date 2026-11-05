# Independent parameter grammar and alias review

**Both mechanisms are rejected.** Whole-output counterexamples remain. Bounded
candidate recognition and restored compatibility flags do not waive them.

Reviewer: GPT-6 Astra, high reasoning, Codex harness in T3 Code, new independent
task `838a6cea-7fc2-451c-a6b0-6d978445e767`. The T3-prepared checkout was cleanly
switched to `review/creole-parameters-independent-e6a64f42` at evidence
`bdfeba117c7af6a527fc66b0acda6d8273fa6d74`. Baseline evidence is
`2608469dace1f680a7e90c0895e44e657f8aaf7f` (production `a6e1718c4`), initial
production `3d92268c4aae5f8bfd3a0e271758b6a8e5442e47`, final production
`8ca73318f584dc29d7d8ab1c91f25a21718ca713`. No production, account, baseline,
comparator, skip, allow-list, target, prior review, maker or parent file was
changed. No children, extra worktrees, local Java/javac, SSH, new runner or
shipping operation was used.

The primary review is `../review.json`; the separate alias mechanism review is
`../alias-compatibility/review.json`, referring to the existing precommitted
`../alias-account.json`. This placement avoids copying or rewriting its account.
The existing guard enumerates only direct reviews of top-level `account.json`
records; it does not separately enumerate this subordinate alias record. The
alias pairs are under their mechanism's standing recursive perturbation walk.
No guard-green or acceptance claim is made.

## Actual CLI evidence

Both baseline and final release CLIs were independently built from exact Git
source snapshots into initially empty private targets with jobs 4. The initial
candidate was also rebuilt privately. Every source ran `-pipe` and separately
`--yaml -pipe`, preserving cwd/include/environment contracts through recorded
private mappings. The unchanged strict comparator was rebuilt independently.
These are actual CLI results; the library diagnostics below are separate.

| Membership | Observations | Valid references | Baseline passes | Final passes |
| --- | ---: | ---: | ---: | ---: |
| Final frozen replay | 1992 | 1858 | 910 | 925 |
| checker28 (included above) | 28 | 28 | 21 | 23 |
| maker fresh16 (included above) | 16 | 16 | 1 | 14 |
| New grammar checker | 24 | 22 | 0 | 10 |
| New alias checker | 12 | 12 | 2 | 4 |
| Replay + new checker | 2028 | 1892 | 912 | 939 |
| Six separate golden controls | 6 | 6 | 6 | 6 |

All cohorts, duplicate observations and nonpositives are retained. There are
zero lost passes, zero new process errors and zero new Rust panics in the replay
and fresh memberships. The same 32 retained SVG panics remain. Retained SVG exit
counts are 1605/93/32 for 0/1/101; YAML exit counts are 1622/93/15. Every other
SVG/YAML call exits zero. All **3996** baseline/final SVG byte and exit
comparisons for replay plus six controls reproduce the maker's archived outputs.
Three new quoted-family cases produce malformed Rust XML both before and after;
these remain output failures and are not hidden by the zero process-error count.

All 1992 replay YAML models are semantically and byte equal in this independent
run, as are the 36 new models and six extra golden models. The maker archive's
1991/1992 byte-equality claim is independently verified: `sprites/sprite_multiple`
only exchanges the `user` and `server` mapping order, with identical values.
This is unchanged `DiagramMeta.sprites: HashMap` nondeterminism, tracked as
🎯T14.6.6. Neither run was repeated to obtain a desired order.

## Six regressions and replay coverage

The following actual CLI controls reproduce **6 → 0 → 6** passes for baseline,
initial candidate and final production:

- `activity/act_creole_monospace`
- `creole/creole_font_name_in_activity`
- `creole/creole_font_name_in_class`
- `creole/creole_font_name_in_note`
- `creole/creole_font_name_in_sequence`
- `creole/creole_font_name_in_title`

Every one of their source hashes is absent from all 1992 replay memberships.
They were caught by broad unit/library gates, not by the focused creole-only
checks or replay memberships. That is a demonstrated replay-coverage gap.
Independent initial diagnostics reproduce product **11173/11251** and assisted
**10492/11251**, losing six and five baseline names respectively. The note case
is the product-only loss. Final product **11179/11251** and assisted
**10497/11251** recover them. The final 72/754 failure-name files are byte-identical
to both the fresh baseline diagnostics and the maker's baseline files. All six
models are byte-equal across baseline, initial and final revisions.

## Causal findings

The production diff is confined to `creole.rs`. `text_render.rs`, including its
monospace metric consumer, public APIs, legacy `to_svg_tspans`, sprite grammar,
WBS, class, sequence, parser, fonts/styles and geometry are unchanged. Both
accounts precede their committed drafts and contain no production edit. Git
cannot establish that no uncommitted draft existed earlier. The rejection import
patch is byte-identical. All 44 maker-cited Java files and four additional
checker files match the pinned Java revision. No fixture-shaped branch, blanket
PUA rejection or unexplained new layout constant was found; the moved image
fallback size 14 is inherited and sourced by `AtomImg.create`.

The anchored grammars follow `Splitter`, the actual combined-font command
pattern, `FontStyle` Ubrex and `Pattern2` whitespace/quote semantics. Combined
attributes precede family fallback, repeated attributes retain the last capture,
and failed candidates leave one-character literal fallback. Fresh cases vary
NBSP boundaries, signs/decimals/overflow, reordered repeated attributes, curly
quotes, gradient operands, image/icon grammar options, crossed closers, nesting
and six consumers. All 36 source hashes are distinct from the prior memberships
and all golden sources.

The hidden-family exception is genuine: Java hides the stream, rejects the
hidden marker in the combined color grammar, then accepts the broader family
pattern. `changeFamily` and `UFontImpl.getFamily(SVG)` preserve U+E023. The fresh
nested WBS control passes. Parameter selection produces no hidden fill, size or
background attribute in the valid parsed outputs inspected. URL/title hidden
values and legacy-tspan behavior are separately preserved; the family exception
does not excuse other paths. Valid icons still yield an empty lexical segment
list, although whole icon rendering fails in downstream consumers.

New counterexamples make these remaining obligations concrete:

- Short hex and lowercase full hex serialization differ. Eight digits are Java
  **RRGGBBAA**: `#8044aa77` becomes `#8044AA` with opacity `0.46667`; Rust emits
  the raw value without opacity. Invalid `#GGGGGG` is accepted by Java's candidate
  grammar, then `CommandCreoleColorChange` catches `NoSuchColorException` and
  preserves black. Rust emits the invalid fill. These are failures.
- Background gradient semantics differ from Rust's filter output. Missing-image
  fallback inherits bold in Rust; Java `AtomImg.create` constructs its own plain
  monospaced configuration. Image/icon whole-output failures remain.
- Quoted families can produce malformed Rust SVG because `text_render` writes the
  family into an attribute without escaping it. Java replaces double quotes with
  single quotes. `font_empty_unknown`, `family_slash` and `alias_monospace_padded`
  fail before and after; no new malformed-XML case was introduced in this set.
- Rust's alias normalization trims **quotes only**, not whitespace. Java's
  `FontStack` trims whitespace and double quotes for `Font.decode`, while SVG
  naming/NBSP uses the preserved definition. Padded/trailing family names are
  therefore not established as equivalent by the compatibility patch.
- Only Courier/monospace/Monospaced, after existing quote trimming and ASCII case
  folding, acquire the compatibility flag. Other tested fonts do not. Courier New
  keeps its existing dedicated metric family. Monospace/Monospaced use the
  unchanged surrounding-metric branch, so the flag does not imply mono metrics.
  Logical fonts, mixed-size baselines and note heights still fail whole outputs.

Both maker fresh16 residuals reproduce exactly. `color_forms` differs only in
`#123` versus `#112233`. `font_family` retains logical SVG name differences and
the restored old Monospaced coupling: `bud` width **22.5527 vs 21.6738**, baseline
**106.7344 vs 106.4355**, plus downstream WBS x differences. 🎯T14.15 owns these
shared serialization/metric obligations. They remain failures, not waivers.

## Reference classification and provenance

Fresh references came only through the parent's existing Monkey Man runner,
worker `/Users/marcelo/rustuml-reference/scratch/worker.x45WQ0CO`, with exact
sources, cwd `new`, GMT+11:00, env `{}`, pinned wrapper/headless/en_AU/UTF-8.
Java `71806a23780b04a5ccde2f8ceb5121edad5eb711`; JAR SHA256
`aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
Every stream/hash/envelope/resource/argv contract was independently validated,
including the lossless parent mapping. Java argv is **derived, not process-traced**.
Historical font and Graphviz environment equivalence remains unproved.

All 36 calls exited 0 with zero timeouts. XML, diagram factory, visible text,
stderr and the unchanged comparator establish 34 valid diagrams. `size_zero`
was intended valid but produced an `IllegalArgumentException` runtime-error SVG
from `Slot`'s zero-length interval. `size_overflow` produced a
`NumberFormatException` runtime-error SVG. `color_invalid_hex` was intended
negative but produced a valid SEQUENCE with retained black text. Original intent
labels are untouched. All pairs, including these two nonpositive references,
remain preserved and executable.

## Gates and preservation

The stable broad golden/standing/guard gates ran once, before checker-only pair
registration. Results reproduce the maker:

- Parser 624/624, CLI pipe 15/15, focused creole 91/91.
- Renderer 788/790, with only the original plain-theme class and initial-blank
  source-identity failures; layout 41/41, math 10/10.
- Product 11179/11251 and assisted 10497/11251 library diagnostics; both red.
- Standing 789/1860 versus matching-environment actual CLI 819/1860, with the
  same 30 mapped include/environment discrepancies. Fixed-membership repair is
  804 → 819. The 36 new registrations and their 14 passes are separate growth.
- Guards 4/6; decimal ratchet and review completeness remain red. Clippy fails at
  unchanged `parse/component.rs:72`; workspace fmt fails in unchanged class
  files. Touched `creole.rs` format passes.

Two setup failures are retained. Initial golden hydration failed because the
configured local file transport was disabled; an explicit per-command transport
allow hydrated exact submodule `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`.
The private source snapshot omitted `Makefile`, so the initial library run stopped
at the oracle Makefile test (30 pass, one setup failure), before parser/renderer.
The exact Makefile was restored; only the unrun parser/renderer crates and the
single setup-failed test ran. That oracle test passed. No passing job was re-armed
and no broad golden/standing/guard gate was repeated.

All **10543** files tracked at the supplied prior evidence base, all **9731**
files at the original frozen inventory revision, and rejection imports of
118/96/135/100 files are byte-exact. Maker archive SHA256
`5967d8e45b3075744356d8ac7b92a449525d67d83cf5b7608a13b32883112751`
and all **68078** member hashes/sizes verify. Old reviews and pairs are untouched.

Warning chrome (🎯T14.12), stripe classification (🎯T14.13), plain/code
(🎯T14.14), concurrent sequence mixed-size/note geometry, forced alignment,
Unicode/Omega widths, URL/italic/legacy tspan and negative diagnostics remain
explicit failures. `malformed_dot`, both `shape_direction` syntax-error SVGs and
Batik-limited math (🎯T14.6.5) remain nonpositive. No sibling packet is imported.

The support archive retains raw CLI/reference stdout/stderr, statuses, full
models, memberships, source/reference captures, private cwd resources, commands,
logs and checker scripts. `archive-files.json` and `archive-integrity.json`
record read-back verification. Binaries/targets/source snapshots are rebuildable
and excluded; exact source blob and binary hashes are recorded. Registration
makes 1896 standing pairs; no post-registration broad-green claim is made.

Next work is bounded to the named serialization, font-resolution, image/icon and
consumer mechanisms, with fresh whole-output checks and preserved failure sets.
Replay enlargement must include the six regression controls. No production fix
or target/account rewrite is part of this evidence-only review.
