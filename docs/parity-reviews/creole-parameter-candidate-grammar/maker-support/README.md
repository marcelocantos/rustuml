# Parameter candidate grammar: maker evidence

This is corrective implementation for a **new independent checker**. No maker
acceptance or review attestation is authored. Previous rejected mechanisms and
all their pairs remain unchanged and executable.

The exact clean starting evidence tip was `2608469dace1f680a7e90c0895e44e657f8aaf7f`
(production `a6e1718c4eb693aec99c3a107fd654afb8a4e4e3`). Rejection
`4e2918c608c9845751c937088dd1a68631657316` was imported unchanged as
`05683a971037e7e9eced8c78d1e11dc198f81cca` before drafting. The primary account
was precommitted in `c14bea51a`; its initial production is
`3d92268c4aae5f8bfd3a0e271758b6a8e5442e47`. The introduced alias regression was
then source-accounted before correction in `5652ec397eab87fb9449beefb7e2368179ae1919`.
Final production is **`8ca73318f584dc29d7d8ab1c91f25a21718ca713`**. Only
`crates/rustuml-render/src/creole.rs` changes in production. No text_render,
public API, WBS, class, sequence, parser/model, font/style/metrics, native/layout,
TIM, comparator, baseline, skip, allow-list, target or shipping edits occur.

## Source mechanism and bounded coverage

Java runs candidate matching on the CharHidder stream. Size requires ASCII
digits; color matches its explicit hex/word grammar. Prefix recognition in Rust
previously selected malformed parameters despite conversion failure, consumed
bodies and leaked hidden color markers into fill. Private typed candidates now
require the complete source-defined family grammar before mutation/consumption.
A failed candidate keeps the existing one-character literal fallback, allowing
recognized inner commands to remain reachable.

Named sources are Splitter's size/color/family/image/icon/sprite patterns,
CommandCreoleColorAndSizeChange's **actual** combined-font pattern (different
from Splitter.fontPattern), FontStyle BACKCOLOR Ubrex, CharClassRaw.WORD,
Pattern2 ASCII case handling and `%s`/`%g`, CommandCreoleBuilder ordering,
StripeSimple and the parameter commands. Combined attributes precede broad
family fallback; repeated captures retain the last value for each attribute.
Parameter bodies use the first case-insensitive close or EOL. Hidden recursion,
flush barriers and actual-empty-line projection remain. Existing strict sprite
recognition and URL behavior are untouched. Legacy to_svg_tspans is byte-exact.
No blanket private-use-character rejection is introduced: Java itself accepts
hidden # in broad family payloads, demonstrated by passing `font_fallback`.

The first draft's empty-icon barrier changed an existing lexical projection.
The unchanged `segments_xml_escaping` test caught it; the barrier was removed
while keeping icon eligibility. Initial focused 86/87 and 89/90 logs, original
script/diff/binary hash and complete first replay are retained in `iteration1/`.
The next focused run passed 90/90 before the broad alias finding.

## Introduced regression and correction

The initial grammar draft cleared the monospace flag for every family. That
introduced six actual-CLI lost passes outside the 1992-cohort replay:

| Golden case | Base CLI | Initial candidate CLI | Corrected CLI | Initial library losses |
| --- | --- | --- | --- | --- |
| activity/act_creole_monospace | pass | fail | pass | product, assisted |
| creole/creole_font_name_in_activity | pass | fail | pass | product, assisted |
| creole/creole_font_name_in_class | pass | fail | pass | product, assisted |
| creole/creole_font_name_in_note | pass | fail | pass | product only |
| creole/creole_font_name_in_sequence | pass | fail | pass | product, assisted |
| creole/creole_font_name_in_title | pass | fail | pass | product, assisted |

Initial library product declined 11179→11173 and assisted 10497→10492. The
existing `explicit_font_monospace_inherits_surrounding_metrics` unit failed
65.0215 versus 57.2813. These were introduced regressions, **not residual waivers**.
All initial gate logs/statuses, full initial replay, six source/reference pairs,
CLI models/outputs and binary hashes remain under `initial-broad/` and
`initial-candidate/` in the archive.

Java CommandCreoleFontFamilyChange→FontConfiguration.changeFamily preserves
the named family, face and size. FontStack.getFonts calls Font.decode on trimmed
names; SvgGraphics.java:697–708 maps Monospaced→monospace and uses NBSP for
monospace/Courier. The existing Rust shared resolver consumes the flag for both
spacing and metrics. The correction sets it only for Courier, monospace and
Monospaced, case-insensitive with the resolver's existing single/double-quote
trimming. Other family payloads retain the grammar behavior. The alias account
separates these invariants and records the parent-approved scope.

All six CLI passes are restored. Both complete library failure-name files are
**byte-identical to the baseline** (72 product, 754 assisted names), not merely
equal totals. Baseline diagnostic executables hash-match the prior maker's
pinned-production executables; invocation provenance is retained. The shared
unit expectation is unchanged and green. Final focused creole tests pass 91/91.
No loss is hidden behind gains: `golden-regression-summary.json` separately
records the six/five initial library losses and six restored actual-CLI passes.

## Immutable actual-CLI replay

Every member runs source-only `-pipe` and separately `--yaml -pipe` using
private before/after release targets, jobs 4, the original cwd/include/env
contract and unchanged strict comparator. Duplicates and nonpositives remain.

| Cohort | Entries | Valid | Before CLI pass | Corrected CLI pass |
| --- | ---: | ---: | ---: | ---: |
| retained | 1730 | 1609 | 709 | 709 |
| maker18 | 18 | 17 | 13 | 13 |
| sprites50 | 50 | 49 | 49 | 49 |
| checker38 | 38 | 34 | 26 | 26 |
| fresh15 | 15 | 15 | 13 | 13 |
| fresh 26 | 26 | 25 | 24 | 24 |
| new15 | 15 | 14 | 12 | 12 |
| delimiters4 | 4 | 4 | 0 | 0 |
| focused2 | 2 | 2 | 2 | 2 |
| checker30 | 30 | 26 | 22 | 22 |
| supplement8 | 8 | 7 | 7 | 7 |
| fresh12 | 12 | 12 | 11 | 11 |
| checker28 | 28 | 28 | 21 | 23 |
| fresh16 | 16 | 16 | 1 | 14 |
| **Total** | **1992** | **1858** | **910** | **925** |

Prior 1948 stays 888/1948. Checker28 improves 21→23/28, specifically
`hide_parameter_size` and `hide_parameter_color`. Fresh16 improves 1→14/16.
The original nine cohorts remain 848/1898. Zero lost passes, new process errors
or new panics occur in these memberships; the same 32 retained SVG panics
remain. Retained SVG exits are 1605/93/32 for 0/1/101 and YAML exits 1622/93/15.
Every other replay SVG/YAML invocation exits 0. Separately, all six affected
golden CLI cases pass again. These are measured domains; no universal CLI
no-loss claim beyond them is made. Full-library failure-name sets are unchanged.

All 1992 full YAML semantic models are equal before/after: no UID, identity,
source metadata, object, factory, shape, topology or loss changes. **1991/1992
are byte-identical**. `sprites/sprite_multiple` only swaps the server/user mapping
order; DiagramMeta.sprites is an unchanged HashMap at parser diagram/mod.rs:225.
Both raw streams are retained, without rerunning to obtain matching order.
The six additional golden models are also byte-identical. All 207 available
valid retained and 17 original-maker evaluated-source comparisons remain exact,
a separate diagnostic from original source SVG/model results. No new SourceProbe.

All 16 fresh Java captures are independently classified valid diagrams using
XML, factory, visible diagnostics, stderr and the unchanged comparator. All
exited 0 with empty stderr and no timeout; exit 0 was not the validity criterion.
No image assets were added; image controls exercise rejected grammar.
Historical malformed_dot remains a NumberFormatException error SVG exit 0.
Both shape_direction syntax-error SVGs and three missing-Batik math references
(html_url_math, display_raw_math, s_display_latex_valid) remain nonpositive.
No positive math or global historical font/Graphviz equivalence claim is made.

## Remaining whole failures and handoffs

- `fresh16/color_forms`: the sole differing attribute is Rust fill #123 versus
  Java #112233. HColorSet.parseSimpleColor:118–143 expands short hex; unchanged
  text_render::normalize_color:1444 preserves leading #. Text content/geometry
  match; the whole failure remains executable.
- `fresh16/font_family`: final output again includes the old Monospaced metric
  coupling, in addition to logical SVG font names. Its `bud` width is22.5527
  versus Java21.6738; baseline 106.7344 versus 106.4355; downstream WBS x positions
  differ. Names remain Monospaced versus monospace and SansSerif versus sans-serif.
  Initial candidate geometry matched but lost the six other controls. The final
  alias correction restores the existing resolver contract. text_render.rs:760
  groups monospace and Monospaced into surrounding metrics; parent assigned the
  coupling to 🎯T14.15. UFontImpl.getFamily(SVG):104–123 and SvgGraphics:697–708
  supply logical-name provenance. No WBS geometry adjustment or text_render edit.
- `checker28/hide_raw_regions`: plain/code registry/reset remains deferred and
  failed, as explicitly accepted by parent. No broad style registry expansion.
- `hide_closing_capture` and `flush_eol_nested`: exact text atoms but sequence
  note-height rounding remains failed; parent sequence owner. Existing mixed-size
  baselines 93.6777 versus 106.2461/105.2793 and html_parameters 78.3672 versus 95.7695
  remain. No lexical/WBS compensation.
- All four sequence SECTION_HEADER/ASTERISK_PREFIXED_LINE delimiter controls,
  forced alignment, warning chrome(🎯T14.12), stripe classifier(🎯T14.13),
  Unicode/Omega/font/PLAIN, unknown/guillemet, note sprite resolver,
  defined class/sequence/scale-color-note registry, URL/italic/legacy tspan,
  welcome and negative/property/surrogate/sequence panic dependencies remain.
  Defined sprites are never classified as missing; all 49 valid sprite controls
  and passing defined note/deployment cases remain passing.

`dependency-handoff.json` carries exact original markup_literal/markup_scan
source/reference paths and pinned Java class parser provenance. The tested TIM8
trace is not generalized to unintegrated class candidate
`68ab512be964e243ae1943f7a9fefd3537091401`, which has quoted-label generic splitting
and positioned-stripe caller work. No sibling imports or combined correctness
claim. Source-line-zero fallback and style_nested remain visible dependencies.

`component-observations.json` and `text-observations.json` preserve runs, styles,
space/rectangle counts and coordinates separately from whole-output verdicts.
Image loading, icon resolution, recognized-invalid color names and oversized
numeric conversion retain their existing separate contracts; no intentional
Rust panic or negative error-contract change. The named source/byte hashes and
both precommitted accounts bound the claim to candidate eligibility and alias
compatibility.

## Corrected broad gates

The initial stable gate run exposed the regression and is preserved. After the
source-accounted correction, focused tests, affected goldens and all memberships
were rerun, then the broad gates ran **once on the corrected stable code**.
No repeat for evidence packaging, threshold changes or unrelated test cleanup.

- Layout 41, math 10, oracle 31, parser 624 and CLI pipe 15 pass.
- Renderer 788/790: only the two existing plain-theme class and initial-blank
  source-identity failures. Their actual CLI before/after inputs remain equal.
- Main product **library diagnostic** 11179/11251; assisted 10497/11251 eligible
  (12550 total, 1299 skipped). Both red; these are not actual-CLI truth counts.
- Standing 789/1860; matching-env actual CLI 819/1860 (before 804), all mapped.
  Same 30 environment/include discrepancies. Registration growth is separate
  from repair. All failed pairs remain registered, including fresh 2 failures.
- Guards 4/6, decimal ratchet/review completeness red. Clippy fails on unchanged
  parse/component.rs:72 dead code; workspace format on unchanged class files.
  Owned creole format and diff checks pass. No maker checker-attestation.

## Provenance, preservation and reproduction

All 9731 historical inventory files and rejection imports 118+96+135+100 files
are byte-exact. All 10543 parity-review/perturbation files tracked at the exact
base are byte-compared against Git. Rejection 0f00130a and every later review
remain unchanged. No previous evidence or membership is rewritten.

Java 71806a23780b04a5ccde2f8ceb5121edad5eb711;
JAR SHA256 aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053.
Wrapper: `/Users/marcelo/rustuml-reference/bin/reference --cwd <worker>/new --timezone GMT+11:00 -- -pipe -tsvg`.
Worker `/Users/marcelo/rustuml-reference/scratch/worker.z0UNQVMV`; env{}.
Java argv is derived from the pinned wrapper, **not process-traced**.
Fresh Java used only parent's existing Monkey Man relay; no local JVM/javac,
SSH, new runners or children.

Original bundle SHA256 6ed6a2a11b796bdea72a3ccbf3558059c69dc9962e7acd6f45c3a057a883ce50;
returned gzip a3b538e932791d9fd6bd1c323f3f365a3376276b43cbc79d6ace094f2be74393.
`relay-envelope-hashes.json` and `relay-lossless-mapping-validation.json` verify
transport, raw response, exact embedded metadata and the lossless files.base64
→ resources.data mapping. Original request/manifest/bundle and both parent
validation records remain intact. `reference-validation.json` records independent
source/stream/member/argv/cwd/TZ/env/pin checks and classification.

The archive retains raw streams, full YAML, immutable/frozen memberships,
sources/references, comparator outputs, initial/corrected logs, scripts and
provenance. Rebuildable binaries/targets/bytecode are excluded; their exact
hashes and commands are recorded. `archive-files.json` enumerates every member;
`archive-integrity.json` records full read-back SHA verification.

Extract into a private cache, restore or losslessly remap absolute resource paths
and build recorded revisions with jobs 4. Run `python run_replay.py before` or
`after` with explicit cohort names from the table (requires PyYAML), plus
`golden-regressions` for the six separate controls. `validate_fresh.py` validates
raw relay captures without running Java. Refer to exact argv/env/statuses in
`gates.json` and build/diagnostic provenance. Stop for a **NEW independent checker**;
this packet does not accept TIM8 or any previous rejected mechanism.
