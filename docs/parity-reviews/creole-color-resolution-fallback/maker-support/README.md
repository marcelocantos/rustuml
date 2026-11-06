# Color resolution and image fallback: corrective maker evidence

This packet stops for a **new independent checker**. It contains no maker review
or acceptance attestation. Every earlier rejection, original pair and whole
failure remains visible.

Start: exact clean `bdfeba117c7af6a527fc66b0acda6d8273fa6d74`, production
`8ca73318f584dc29d7d8ab1c91f25a21718ca713`. Independent rejection
`f6c84a6215e734955890f6058c309423cf228f1d` was imported unchanged as
`756f7a41db41a373e97aa7714cab22b634721354` before drafts. Both new accounts and
the alpha scope handoff were committed first in
`f977806cef5a90a868175e8e707a89d4d791d29d`. Production is
**`1ffb930b7b56decb15208121db5e70d252ed3c1b`**, changing only `creole.rs`.

## Source mechanisms

Candidate syntax and color resolution are separate invariants. The prior strict
parameter grammars, hidden stream, command barriers, first-close/EOL handling,
font alias predicate, public APIs and legacy tspan prefix are unchanged.

`CommandCreoleColorChange.executeAndGetRemaining:80–98` catches
NoSuchColorException and analyzes the selected command body using the current
font configuration. Thus `#GGGGGG` is a legal candidate but an unresolved color;
it does not become a literal tag or an invalid fill. Rust now leaves the current
fill unchanged on this failure, while preserving command consumption and flushes.

`CommandCreoleStyle.getExtendedColor:82–88` instead calls
`HColorSet.getColorOrWhite`. A selected unresolved BACKCOLOR becomes white.
These are separate behaviors. The private predicate follows HColorSet's simple
hex widths/digits, special names, named lookup and gradient operand validity.
It reuses the existing named lookup without changing it; the seven Java
Archimate names and Java's absence of CSS4 RebeccaPurple are explicitly accounted
for from ColorTrieNode. Valid spellings keep their existing emission path.
This predicate does not normalize valid colors, implement gradients or expand
candidate syntax. Invalid gradient operands resolve white; valid gradients
remain a separate rendering failure.

`CommandCreoleColorAndSizeChange:82–105` has a distinct throwing contract. Both
fresh invalid-font cases produce NoSuchColorRuntimeException crash SVGs, with
matching stderr and exit 0. `UgDiagram:92–103` catches Throwable, reports it and
uses CrashReportHandler.exportDiagramError. They are **nonpositive**, not valid
color-fallback requirements. Rust still emits an ordinary SVG; that negative
contract mismatch is preserved. parse_segments returns Vec<Segment>, handle_tag
returns bool and render_svg returns String. Correct propagation/serialization
requires a coordinated error channel beyond creole-only ownership. No invented
fallback, intentional Rust panic, error-string synthesis or API change is added.
`negative-font-contract.json` gives the durable source/ownership handoff.

`AtomImg.create:114–115` constructs a new
`FontConfiguration.blackBlueTrue(UFontFactory.monospaced(14))`. The fallback is
plain, black, monospaced, normal-position text, independently of surrounding
markup. Rust now builds this fallback Style from defaults rather than cloning
nested bold/italic, fill, background, family, position and decoration. Surrounding
runs and command barriers restore their original styles. Image loading, fallback
message selection, image/icon rendering and scale geometry are unchanged.

The explicit TextBase bold/italic OR in shared measurement/emission remains an
inability to reset consumer base styles. It is an explicit consumer-side handoff,
not compensated in creole. The fresh case named `image_base_bold` passes; its name
expressed test intent and does **not** establish that an explicit bold TextBase
override was reached. No acceptance of all base-style consumers is claimed.

## Alpha scope stop

No alpha code was drafted. HColorSet.parseSimpleColor:145–153 decodes eight
hex digits as RRGGBBAA into XColor; HColor.toSvg and DriverTextSvg carry it to
SvgGraphics.fillMe:891–898, which emits six-digit fill and alpha/255 as five-decimal
fill-opacity. `#8044aa77` therefore becomes `#8044AA`, opacity `0.46667`.
Rust Style has no opacity field, and write_text_element emits no opacity. A parse
change alone cannot reproduce this. The durable `../alpha-scope-handoff.json`
names the shared serialization/API boundary under parent-owned 🎯T14.15,
alongside short hex, named colors, logical-family metrics/escaping and alias
whitespace. No attributes are injected through fill text.

## Actual CLI replay

Every immutable membership, duplicate and nonpositive runs source-only `-pipe`
and separately `--yaml -pipe` through private before/after release binaries,
jobs 4, with its cwd/include/environment contract and unchanged strict comparator.

| Cohort | Entries | Valid | Before CLI pass | After CLI pass |
| --- | ---: | ---: | ---: | ---: |
| retained | 1730 | 1609 | 709 | 709 |
| maker18 | 18 | 17 | 13 | 13 |
| sprites50 | 50 | 49 | 49 | 49 |
| checker38 | 38 | 34 | 26 | 26 |
| fresh15 | 15 | 15 | 13 | 13 |
| fresh26 | 26 | 25 | 24 | 24 |
| new15 | 15 | 14 | 12 | 12 |
| delimiters4 | 4 | 4 | 0 | 0 |
| focused2 | 2 | 2 | 2 | 2 |
| checker30 | 30 | 26 | 22 | 22 |
| supplement8 | 8 | 7 | 7 | 7 |
| fresh12 | 12 | 12 | 11 | 11 |
| checker28 | 28 | 28 | 23 | 23 |
| fresh16 | 16 | 16 | 14 | 14 |
| golden-regressions | 6 | 6 | 6 | 6 |
| checker36 | 36 | 34 | 14 | 15 |
| fallback16 | 16 | 14 | 1 | 11 |
| **Total** | **2050** | **1912** | **946** | **957** |

The previous 1992 observations remain 925 passes; six golden regression controls
remain 6/6. Checker36 improves 14→15/36 (34 valid), specifically
`color_invalid_hex`. Compact fallback16 improves 1→11/16 (14 valid). The original
nine-cohort 848/1898 result, checker28 23/28 and previous maker16 14/16 are retained.
There are zero lost passes, zero new process errors and zero new Rust panics.
The same 32 retained SVG panics remain. Retained SVG exits are 1605/93/32 for
0/1/101; YAML exits are 1622/93/15. Every other CLI call exits zero.

Malformed SVG output is recorded separately from process errors. The three
checker quoted-family cases still produce malformed XML before/after; they are
not concealed by exit zero. No new malformed-XML case appears.

All **2050 full YAML models are semantically and byte-identical** in this run.
There is no UID, identity, object, source, factory, shape, topology, serialization
or loss change. Earlier sprite HashMap-order observations remain intact under
🎯T14.6.6; no rerun was used to obtain equality. The available 207 valid retained
and 17 original-maker evaluated-source comparisons remain exact, as separate
diagnostics from original-source SVG/YAML. No new SourceProbe was requested.

Fresh Java classifications use raw XML, diagram factory, visible diagnostic
text, stderr and the unchanged comparator, not exit status or intended labels.
All 16 calls exit zero; the two font-color throws have nonempty stderr. Fourteen
are valid diagrams and two are runtime-error SVGs. All source/reference pairs,
including both nonpositives, are registered. Checker36 independently retains 34
valid plus size_zero IllegalArgumentException and size_overflow NumberFormatException
crash SVGs. Its intended-negative invalid hex remains a valid diagram.

## Component effects and retained failures

The fresh passing cases cover invalid plain/current colors, EOL activity labels,
white backgrounds, nested backgrounds/current fill, WBS/member text, malformed
neighbor grammar and three image fallback style configurations. Existing plain
image fallback stays passing. Full runs/styles/coordinates and counts are in
`component-observations.json` and `text-observations.json` in the archive,
separate from whole-output verdicts.

Three fresh valid whole failures remain:

- `color_valid_neighbors`: only fill `#123` versus Java `#112233`.
- `color_registry_boundary`: BUSINESS versus `#FFFFCC` and `#red` versus
  `#FF0000` serialization. They are resolvable Java colors; treating them as
  invalid would be an incorrect fallback. RebeccaPurple now correctly retains
  current color, but that does not make the combined case a whole pass.
- `back_invalid_gradient`: both invalid operands now use white; the valid
  red-blue gradient still differs from the existing Rust filter output.

Checker `image_nested` now has the correct plain black fallback. Its remaining
second-line baseline is 85.9131 versus Java 85.3535. `image_grammar` retains
96.6367/111.9473 versus 95.6504/110.9609 on the later literal lines. These remain
sequence consumer whole failures. All icon consumer failures remain. No geometry
retuning or sibling sequence import occurs.

Previous short-hex/logical-name/Monospaced coupling failures remain exactly
visible, as do quoted-family malformed SVG, alias whitespace, gradients,
warning chrome(🎯T14.12), stripe classifier(🎯T14.13), plain/code(🎯T14.14),
sequence mixed-size/note height, forced alignment, Unicode/Omega/font/PLAIN,
unknown/guillemet, note sprite resolver, defined class/sequence/scale-color-note
registry, URL/italic/legacy tspan, welcome, negative/property/surrogate and
sequence panic dependencies. Defined sprites are never classified as missing;
all 49 valid sprite controls and defined note/deployment passes remain.

`dependency-handoff.json` retains exact original markup_literal/markup_scan
source/reference paths and Java class-parser provenance. This tested tree is not
unintegrated class candidate `68ab512be964e243ae1943f7a9fefd3537091401`, with its
quoted-label generic splitting and positioned-stripe caller changes. No combined
correctness claim, source rewriting or renderer-synthesized class objects.
Source-line-zero and style_nested remain separate failures.

Historical malformed_dot, both shape_direction syntax-error SVGs and the three
Batik-limited math references remain nonpositive. No math or global historical
font/Graphviz equivalence claim is made.

## Tests and stable broad gates

The focused run first had a new-test identifier typo (strip_markup instead of
stripped_text), then two new tests incorrectly assumed balanced same-name closes.
They were corrected to Java first-close semantics without changing production.
The existing image unit expected inherited fill; its expectation now names
explicit black, justified by AtomImg.blackBlueTrue. The failed logs and original
draft script are preserved. Final focused creole tests pass **94/94**. The alias
resolver and its existing unit expectations are untouched.

The full broad suite ran once after stable code and all fresh registrations.
Exact argv/cwd/environment/status/timing are in `gates.json`; no broad rerun for
packaging, no baseline or threshold changes, no unrelated unit cleanup.

- Layout 41, math 10, oracle 31, parser 624 and CLI pipe 15 pass.
- Renderer 791/793: the same plain-theme class and initial-blank source-identity
  failures. Their actual CLI control inputs remain unchanged before/after.
- Main product **library diagnostic** 11179/11251 and assisted 10497/11251
  eligible (12550 total, 1299 skipped). Both red, and neither is actual-CLI truth.
  Both complete failure-name files are byte-identical to exact baseline,
  with 72 product and 754 assisted names. No losses or compensating gains hide
  behind totals.
- Standing 815/1912; matching-env actual CLI 845/1912, before 834. All paths map;
  the same 30 include/environment discrepancies remain. Registration growth is
  distinct from repair and all failing pairs remain executable.
- Guards 4/6, decimal ratchet and review completeness red. Clippy fails at
  unchanged parse/component.rs:72 dead code. Workspace format fails in unchanged
  class files; owned creole format and diff checks pass.

## Preservation and reproduction

All 9731 original inventory files, earlier rejected import files 118/96/135/100,
and the newly imported 120 files are byte-exact. All **10721** parity-review and
perturbation files tracked at exact starting evidence bdfeba117 were compared
by bytes against Git. No old account, review, pair, baseline, comparator, skip,
allow-list or target changed. No maker-authored review.json exists for this packet.

Pinned Java `71806a23780b04a5ccde2f8ceb5121edad5eb711`, JAR SHA256
`aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
Wrapper `/Users/marcelo/rustuml-reference/bin/reference --cwd <worker>/new --timezone GMT+11:00 -- -pipe -tsvg`;
worker `/Users/marcelo/rustuml-reference/scratch/worker.upgz9vBD`, env `{}`.
Java argv is derived from wrapper construction, **not process-traced**. Only
parent's existing Monkey Man relay executed fresh Java; no local JVM/javac,
SSH, new runners or children. No assets were added.

Original bundle SHA256
`583c8cf20bafa6893d0ae23f7128fc977190400b95d1bf11217b33c9bb0f1c33`;
returned gzip `6f76972eead6770aede07d7bdcf46c61a50d6354cea834545504367176527d20`.
The untouched request/manifest/files envelope, lossless parent resources mapping,
exact embedded metadata, transport, both parent validation records and raw streams
are retained. Independent hash/resource/member/stream/argv/cwd/TZ/env validation
is recorded separately from validity classification.

`measurements.tar.gz` retains every raw SVG/YAML/stderr/comparison, command status,
frozen and original membership, fresh source/reference, log and script.
Rebuildable binaries/targets/bytecode are excluded with source/binary hashes and
build commands recorded. Earlier cwd resource contracts and raw Java histories
remain in unchanged prior archives. `archive-files.json` enumerates member hashes;
`archive-integrity.json` records complete read-back verification.

Extract to a private cache, restore or losslessly remap absolute resource paths,
rebuild the recorded revisions with private targets/jobs4, and run
`uv run --no-project --with pyyaml python run_replay.py before` (or after) with
explicit cohort names from the table. `validate_fresh.py` validates returned
captures without Java. These are maker observations for a **NEW independent
checker**, not acceptance of this packet, either previous mechanism or TIM8.
