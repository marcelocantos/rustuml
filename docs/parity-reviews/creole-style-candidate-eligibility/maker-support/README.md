# Style recognition and WBS corrective maker packet

Implementation `8df6bb53fac4fdf364a1a9aeea0b203cb6dab8e6` corrects supported
style-candidate eligibility and WBS display/shape text components against exact
base `a84856637adcd8e8d4e78fd2ee00658c9c612feb`. Independent rejection
`521fe226d428c01128f4379bb7342584772924d7` was imported unchanged first as
`dc796484a0a1ef723b8030ef0fd767da2708533d`. This is maker evidence for a NEW
independent checker, not self-review, acceptance, a re-review of TIM8, or a waiver
of any whole-output failure. No sibling production was imported.

Five preimplementation causal accounts precede their respective drafts:

- `829154e5b`: `creole-style-candidate-eligibility`,
  `wbs-display-line-projection`, `wbs-idea-shape-block-selection`.
- `52203dbe7`: `wbs-leaf-component-origin`, discovered by the first maker pass.
- `3da745665`: `creole-delimited-style-candidate-eligibility`, the existing
  delimiter branch of the same Java command mechanism.

Production touches only renderer `creole.rs`, `wbs.rs`, and the additionally
authorized parser `diagram/wbs.rs`, `parse/wbs.rs`. Shared public text APIs,
`text_render.rs`, legacy `to_svg_tspans`, class, sequence, TIM, fonts/metrics,
styles, native/layout, registries and verification policy remain unchanged.

## Causal coverage

Java `CommandCreoleBuilder` orders closed HTML style candidates before EOL
candidates. `CommandCreoleStyle.matchingSize` returns captured content length;
empty closed bodies therefore fall through, while size/color commands test
parameter groups. Rust now selects an eligible candidate before consuming its
body, handles supported closing aliases and escaped candidates, recurses, and
restores style with command flush boundaries. Empty closing tags remain visible
styled text; genuine consumed-empty stripes still use the unchanged base-style
space projection. Delimiter candidates require nonempty bounded captures and
have no EOL fallback. Failed candidates consume literal characters without
swallowing later inner sprite commands.

Java `WBSDiagram.addIdea` builds `Display.getWithNewlines` before its text block.
The WBS caller now projects and caches the same line content, height and baseline
for max-width/summed-height measurement and paint, including leading/interior/
trailing blanks. Raw model labels retain their line escapes. Raw URL/math
regions and escaped backslashes do not accidentally split. Forced-alignment
`\r`/`\l` and unrelated Display features are not claimed by this bounded port.

`CommandWBSItemNew/Old` feed `IdeaShape` to `WBSTextBlock.buildMain` and `ITFLeaf`.
The parser now extracts adjacent underscore shape syntax into `WbsShape::None`;
whitespace-delimited literal underscores stay in labels. The consumer selects
boxed style padding 10 (pinned `plantuml.skin`) or unboxed margins left/right/
top/bottom 0/3/1/1 (`TextBlockUtils.withMargin`). `ITFLeaf.drawU` paints its block
at the supplied origin; composed nodes retain their existing trunk offset.
This distinction fixes the newly exposed tiny unboxed-leaf origin without
changing topology or spacing algorithms. No renderer underscore stripping occurs.

`causal-coverage.json`, the precommitted accounts and `java-source-provenance.json`
name the Java/Rust data flow. `text-observations.json` and
`component-observations.json` quantify text runs, styles, spaces, baselines,
canvases and rectangles separately from whole-output comparison. For example:

- `empty_bold_recognition`: two message text runs become three, matching Java's
  `A<outer`, bold `</b>>B`, and `C`; canvas width 179 becomes 217.
- `present_bold_empty`: three size-13 spaces become `</b>`, `</i>`, size-26
  `</b>`. Canvas 145×153 becomes Java's 173×169. The last baseline still fails
  in the sequence consumer (93.6777 versus 106.2461); this is not a whole pass.
- `blank_restored`: three base-size spaces and all baselines stay exact.
  `wbs_boxless_empty`: two base-size-12 space atoms and visible bold closing
  text match Java; the whole output now passes.
- `positioned_fallbacks`: each of five unclosed style prefixes now allows the
  inner missing sprite to be consumed, yielding two text runs rather than a
  literal sprite; escaped closing delimiters match Java. Whole output passes.

These coordinates describe observations, not constants or repair conditions.

## Actual source-only CLI outcomes

Private before/after release binaries (jobs 4) received each exact original
source through `-pipe` and separately `--yaml -pipe`. The diagnostic calls the
unchanged strict comparator. Immutable memberships include duplicates and
nonpositive references; they were not replaced with successful subsets.

| Cohort | Entries | Valid | Before pass | After pass |
| --- | ---: | ---: | ---: | ---: |
| Retained TIM/checker8 | 1730 | 1609 | 709 | 709 |
| Original maker | 18 | 17 | 13 | 13 |
| Existing sprite controls | 50 | 49 | 49 | 49 |
| Prior rejected checker | 38 | 34 | 26 | 26 |
| Prior corrective maker | 15 | 15 | 13 | 13 |
| New rejection checker | 26 | 25 | 19 | 24 |
| Style/WBS maker | 15 | 14 | 3 | 12 |
| Delimiter maker | 4 | 4 | 0 | 0 |
| Mid-line delimiter maker | 2 | 2 | 0 | 2 |

There are **zero lost passes** across all 1898 observations. Five of the prior
six valid checker failures now pass: empty bold recognition and four WBS cases.
The sixth, `present_bold_empty`, remains an executable failure. Original first
five cohorts retain all results. Retained SVG exits remain 1605 exit-0, 93 exit-1,
32 exit-101 panics; YAML remains 1622 exit-0, 93 exit-1, 15 exit-101. Other cohorts
have only CLI exit-0 outcomes. No new CLI errors or panics occurred. All raw
loss/error/panic outputs and stderr streams remain in the archive.

All 207 available valid retained and 17 original-maker evaluated-source
comparisons remain exact. No new SourceProbe was run for these 21 new sources;
whole original-source CLI output and YAML are distinct diagnostics.

All 21 new Java calls exited zero with no timeouts. Independent classification
finds 20 valid diagrams and one `malformed_dot` NumberFormatException error SVG.
That exit-0 exception is nonpositive, never a valid pass or a requirement to
panic Rust. Earlier malformed scales, welcome and syntax outcomes keep their
original classifications and bytes.

## Model and identity effects

Full parsed YAML trees change only in five WBS diagrams: old checker
`wbs_boxless`, and new `wbs_boxless_tree`, `wbs_shape_color`,
`wbs_literal_underscore`, `wbs_boxless_empty`. Twelve nodes gain explicit
`shape: None`; their labels lose only command shape/color syntax. One color
field becomes `#pink` instead of remaining in label text. Node counts, child
order, depth, side, UID/identity, diagram factory and source metadata do not
change. Display newline projection does not rewrite the model label.

Old models lacking shape deserialize as Box; Box serialization omits the field,
so plain-model serialization stays compatible. Focused JSON roundtrips cover
both omitted Box and explicit None. All other full YAML trees are equal; the
single unrelated sprite-map byte difference is serialization order. Exact
changed paths/values are in `model-observations.json` and
`model-effects-summary.json`, rather than a blanket unchanged-model claim.

## Residuals and separate owners

`dependency-handoff.json` preserves every previous obligation and exact original
source/reference paths for both markup controls, with pinned Java class parser
provenance. This tested class parser still splits label generics only when
label-derived-from-id. Unintegrated candidate
`68ab512be964e243ae1943f7a9fefd3537091401:parse/class.rs:993` already splits quoted
labels and has positioned-stripe caller work. Neither combined sprite/generic
nor positioned-class correctness has been measured. Both original markup
failures, `style_nested`, and source-line-zero remain visible failures.

Additional retained whole failures:

- Sequence mixed-size baseline: `present_bold_empty` and
  `style_parameter_contrast`. The latter's size-25 baseline remains 93.6777
  versus Java 105.2793. Shared content/styles and canvas match; whole SVG fails.
- Sequence stripe preprocessing: all four original delimiter controls.
  Java `CreoleStripeSimpleParser.SECTION_HEADER` and
  `ASTERISK_PREFIXED_LINE_PATTERN` run before shared command parsing. Empty
  `----` is a section stripe, five hyphens are a struck hyphen, and leading
  `**` invokes list behavior. Rust's sequence `whole_strike_label_inner` and
  missing list-stripe processing differ. The two NEW mid-line controls isolate
  shared commands; they do not replace these four failures or waive them.
- `wbs_shape_color`: Java `CommandWBSItemOld` emits “Please define
  Direction/Shape before Color/Id.” warning decoration. Corrected shape/color
  models do not implement warning metadata or decoration; whole SVG fails.
- Unknown/guillemet, Unicode/font/PLAIN, note resolver, defined class/sequence/
  scale-color note registry, URL recursion/italic/legacy tspan, nested quoted
  class, welcome, negative diagnostics, property/surrogate, and existing
  sequence panics remain. Defined sprites are never classified as missing;
  defined basic note/deployment and all 49 valid existing sprite controls pass.

These are failure handoffs, not scope waivers or acceptance claims.

## Final gates (one stable run)

`gates.json` retains exact argv, cwd, environment, elapsed time and exit status;
raw logs are archived. No broad gate was repeated for evidence-only work.

- Layout 41, math 10, oracle 31, parser 624 and CLI pipe 15 pass.
- Renderer 782/784: unchanged plain-theme class-default and initial-blank
  source-identity failures. Actual CLI SVG/YAML captures of both existing unit
  inputs are byte-identical before/after.
- Main product **library diagnostic** 11179/11251, assisted library diagnostic
  10497/11251 eligible (12550 total, 1299 skipped). Both exit 101 with prior
  failures. These are not actual-CLI truth numbers.
- Standing 712/1766; matching-environment CLI 742/1766 (before 726).
  Old 1745 membership contributes 698 harness / 728 CLI, up five from
  693 / 723. New 21 registration contributes 14 passes (before CLI 3).
  All paths map; the same 30 environment/include discrepancies remain.
- Guards 4/6; decimal ratchet and review-record guards remain red.
- Clippy fails on unchanged `parse/component.rs:72` dead code. Workspace fmt
  fails in unchanged parser/render class files. All four touched files format
  cleanly; diff check passes. No unrelated cleanup or baseline changes.

The initial draft accidentally changed a legacy arm and failed compilation;
that arm was restored before successful shared tests. Its failed log is retained.
An interrupted final build was resumed; its already passing focused test was
not repeated. Both draft iterations, commands, measurements and interruption
record remain. Only final stable code was committed.

## Provenance, preservation and reproduction

Pinned Java source `71806a23780b04a5ccde2f8ceb5121edad5eb711`; JAR SHA256
`aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
Only the parent's existing Monkey Man runner executed Java. There was no local
Java/javac, SSH, new runner/agent or shell-created worktree. Every source file
cited by the provenance record was checked byte-for-byte against the pin.
Wrapper: `/Users/marcelo/rustuml-reference/bin/reference --cwd PRIVATE_CWD
--timezone GMT+11:00 -- -pipe -tsvg`. Returned raw argv/cwd/env, resource and
stream hashes, memberships and immutable original bundles were independently
validated. Java argv is derived from the pinned wrapper, **not process-traced**.
The parent lossless files.base64 → resources.data mapping embeds exact original
request/manifest bytes. Both parent mapping records and raw envelopes are retained.
No global historical font/Graphviz environment equivalence claim is made.

Returned gzip hashes:

- Maker15: `f6df92fe5ad2e29c114b86f9b31d11d26386d3d56ed703fbc9b6e4cc086b4ab9`.
- Delimiters4: `e44910b8802632c7fa7498115ea349da21422338f45c668599d31574b4c71fac`.
- Focused2: `994b171b0691801319589f27de2a54fce55dd536efad45d001de73f02db38458`.

All 9731 historical inventory files, 118 earlier rejected-evidence files, and
96 files from the newly imported rejection retain exact bytes. Prior accounts,
reviews and pairs, including rejection `0f00130a`, are untouched. Fresh pairs
are registered under `test-diagrams/perturbations/creole-style-candidate-eligibility/`
including all seven new nonpassing outcomes (six valid failures, one exception).

`measurements.tar.gz` contains immutable memberships/inputs/references, every
before/after SVG/YAML/comparator stream and command result, all Java transport
captures, two intermediate iterations, scripts and logs. Rebuildable binaries,
targets and bytecode caches are excluded. `archive-files.json` hashes each
member; `archive-integrity.json` records read-back verification. Binary/source
hashes and exact build commands are in `build-provenance.json`.

Extract into a private cache, rebuild exact revisions with the recorded private
targets/jobs4, and run `uv run --no-project --with pyyaml python run_replay.py
before retained maker18 sprites50 checker38 fresh15 fresh26 new15 delimiters4
focused2` (or `after`). Honor each recorded cwd/include/env contract and restore
recorded cache paths or map them losslessly. `validate_fresh.py` and the two
supplement validators inspect archived Java returns without executing Java.
Historical external resources/cwd captures remain in the unchanged earlier
archives. No self-authored checker attestation is included.
