# Recursive character hiding and parameter flush maker packet

Production `a6e1718c4eb693aec99c3a107fd654afb8a4e4e3` changes only
`crates/rustuml-render/src/creole.rs`, against exact clean evidence tip
`a3afe487a66ae26c73e2f1853d6c26fc2d228875`. Independent rejection
`5ec48be56c4bce12ec646c29ed8e6d4be5e8518b` was imported unchanged first as
`f75103e509a5c5cd61341d78421a436f7103603f`. New causal accounts for both
mechanisms and the warning research handoff were committed before drafts in
`5d039c21c9374ee91904fe4da430c22ff6e2c4f3`.

This is corrective maker evidence, not independent review or acceptance of any
of the five rejected mechanisms. Every previous account, review, rejection and
pair remains unchanged. A NEW independent checker is required. No shipping,
target edits, children, local Java/javac, SSH or new runner occurred.

## Source-derived change

`StripeSimple.analyzeAndAdd` applies `CharHidder.hide` at every recursive stripe
entry, before command candidate matching. Backslash-tilde hides the tilde and
consumes the backslash. Tilde consumes exactly one following character, hiding
only Java's listed syntax characters. The source-defined E000..E0FF encoding
keeps those characters inert during matching and recursion. `AtomText` restores
them when constructing a literal atom.

Rust now follows those same boundaries: `walk_segments` creates the hidden
stream; both candidate scanners and recursive painting consume it;
`push_literal` restores hidden characters before XML escaping. The old
scanner-only escape skipping and separate tilde painting arm are removed.
There is no second ad-hoc escape pass, no public API change and no legacy
`to_svg_tspans` change. Escaped overlapping openers now consume one character,
so `q~***x**r` produces literal `q*`, bold `x`, and `r`.

`StripeSimple.modifyStripe` flushes pending text before every selected command,
and recursive processing flushes at body end. Supported size/color arms now
use the existing private barriers around their bodies, including empty bodies
and equal-style nesting. Barriers are removed from drawable output; they prevent
coalescing across commands. An entirely consumed actual stripe still projects
the restored base-style space; lexical `parse_segments("")` remains empty.
No blanket padding is added to absent models.

WBS display projection, model/shape parsing, leaf origin, geometry and serialization
are unchanged. The checker-supported arithmetic was not retuned to compensate
for lexical problems. `text_render.rs`, public text APIs, class, sequence,
fonts/styles/metrics, native/layout, TIM, registries and test policies are untouched.
Named source locations, data flow, perturbation axes and hashes are in the two
precommitted accounts, `causal-coverage.json`, and `java-source-provenance.json`.

## Actual CLI observations

Each immutable source ran through privately built before/after release CLI
binaries using `-pipe`, separately `--yaml -pipe`, with its original cwd/include/
environment contract. Builds use separate private targets and jobs 4. The
unchanged diagnostic calls the unchanged strict SVG comparator. Duplicates and
nonpositive members remain in the denominator and archive.

| Cohort | Entries | Valid | Before pass | After pass |
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
| Newly imported checker initial | 30 | 26 | 20 | 22 |
| Newly imported checker supplement | 8 | 7 | 1 | 7 |
| Fresh hiding/flush maker | 12 | 12 | 1 | 11 |
| Total | 1948 | 1814 | 870 | 888 |

The original nine cohorts remain **848/1898**. The 38 checker pairs improve
**21→29/33 valid**; the 12 fresh pairs improve **1→11/12**. There are zero lost
passes, zero new errors and zero new panics. The same 32 retained SVG panics
remain. Retained SVG exits are 1605/93/32 for exit 0/1/101; YAML exits are
1622/93/15. All other CLI calls exit zero. `loss-error-summary.json` reports
losses, errors and panics separately; every raw stderr, including empty ones,
is retained. No failure is waived.

Full YAML is byte-identical before/after for all 1948 observations, including
error outputs. No model content, UID/identity, topology, factory, source metadata,
shape/default serialization or color changes occur in this correction. All 207
available valid retained and 17 original-maker evaluated-source comparisons
remain exact. There was no new SourceProbe request; these are distinct from
original-source CLI SVG and full-model comparisons.

All 12 fresh Java outputs are independently classified valid diagrams (sequence,
WBS and activity); all exit zero with empty stderr and no timeout. Validity was
not inferred from exit status. The source/resource/member/stream hashes,
wrapper argv, private cwd, GMT+11:00, empty explicit env overrides and pinned
provenance were independently checked.

Five imported checker references remain nonpositive: the two shape_direction
syntax-error SVGs (Java exit 200), and `html_url_math`, `display_raw_math`,
`s_display_latex_valid` with missing-Batik fallback images. The original intended
labels, final classifications and raw reference evidence are preserved. These
are not positive math requirements; no math claim is made. Historical
`malformed_dot` remains a NumberFormatException error SVG returned with exit 0.

## Component effects and residual failures

`text-observations.json` and `component-observations.json` in the archive retain
text runs, styles, baselines, dimensions and space/rectangle counts separately
from whole-output comparison. The eight newly passing checker cases are:

- `html_escape_pairs`, `delimiter_escapes`;
- `s_display_commands`, `s_display_escapes`, `s_leaf_commands`, `s_leaf_escapes`;
- `s_delimiter_escape_overlap`, `s_html_bold_backslash`.

In `s_display_commands` and `s_leaf_commands`, equal-style `q`/`r` atoms now
remain separate at the same positions. WBS dimensions remain 140×191 and
103×144 respectively. No coordinate adjustment produced these gains.
Backslash-tilde text now renders `a~`, and overlapping escaped delimiters retain
Java's exact styled run boundaries across recursive styles and WBS consumers.

Remaining in-scope evidence is not promoted to a whole pass:

- `checker30/html_parameters` now has separate `q`, `r`, `s` atoms matching
  Java. Its size-31 italic baseline remains Rust 78.3672 versus Java 95.7695,
  owned by the separate sequence cycle. Original size-26/25 baselines remain
  93.6777 versus 106.2461/105.2793.
- `fresh12/literal_neighbors` has matching content/styles/escape boundaries,
  including ASCII punctuation and the plain URL neighbor. Its `P~a~ΩQ` width
  is 48.0835 versus Java 51.2446 at size 13: the retained font/Unicode dependency.
  The whole fresh case remains executable and failed.
- `checker30/html_extended` retains unsupported extended activation syntax.
  No new activation forms were added.
- `display_forced_alignment` remains a valid whole failure, with no waiver.
- All four original sequence line-stripe delimiter controls still fail through
  `SECTION_HEADER` / `ASTERISK_PREFIXED_LINE` processing. Their original sources
  were not rewritten and sequence code was not imported or edited.

Old-order WBS warning composition is a **scope stop**, accepted by the parent.
`CommandWBSItemOld:115` calls `TitledDiagram.addWarning`; its merged warning
collection reaches `DiagramChromeFactory12026:138-245`, which measures and
paints a warning banner before the other diagram decorations. This requires a
shared metadata/composition boundary outside creole ownership. The durable
proposal is `docs/parity-reviews/creole-parameter-command-flush/wbs-warning-scope.json`. No warning code was drafted, no WBS model changed,
and no WBS offset was introduced. `wbs_shape_color` and `shape_colors_old`
remain explicit whole failures; this is not a warning acceptance claim.

`dependency-handoff.json` preserves exact original markup source/reference
paths and pinned Java class parser provenance. The tested class code differs
from unintegrated candidate `68ab512be964e243ae1943f7a9fefd3537091401` at its
quoted-label generic splitting/positioned-stripe callers. No combined class or
concurrent sequence correctness is asserted. Original markup, class positioned
styles, source-line-zero, unknown/guillemet, Unicode/font/PLAIN, note resolver,
defined class/sequence/scale-color note registry, URL/italic/legacy tspan,
welcome, negative/property/surrogate and sequence panic dependencies remain.
Defined sprites are not missing sprites; all 49 valid existing sprite controls
and the passing basic note/deployment cases stay passing.

## Stable broad gates, run once

`gates.json` retains exact commands, cwd, environment, timings and statuses.
Raw logs are archived. No broad gate was repeated for packaging.

- Layout 41, math 10, oracle 31, parser 624 and CLI pipe 15 pass.
- Renderer 784/786: the same plain-theme class and initial-blank source-identity
  failures. Both existing unit inputs also have byte-identical actual CLI
  SVG/YAML captures before/after. Focused creole tests pass 87/87.
- Main product **library diagnostic** 11179/11251; assisted library diagnostic
  10497/11251 eligible (12550 total, 1299 skipped). Both remain red and are not
  actual-CLI truth counts.
- Standing 752/1816; matching-environment actual CLI 782/1816 (before 764).
  Original 1766 contributes 712 harness / 742 CLI, unchanged. Checker38 adds
  29 harness/CLI (before CLI 21); fresh12 adds 11 (before CLI 1). Registration
  growth is distinct from repair. All paths map; the same 30 environment/include
  discrepancies remain.
- Guards 4/6, decimal ratchet and review records red. Clippy fails on unchanged
  `parse/component.rs:72`; workspace fmt fails in unchanged class files.
  The touched creole file's fmt and diff check pass. No unrelated cleanup,
  harness/comparator/baseline/skip/allow-list changes or self-attestation.

The first new flush test incorrectly assumed balanced nested size closers.
Java's parameter command captures through the first close. The test was changed
to nested EOL captures to isolate equal-style flush behavior; production was
not changed to fit the mistaken expectation. The initial 86-pass/1-fail log,
original draft script, and corrected 87-pass log remain archived.

## Preservation and reproduction

All 9731 historical inventory files, 118 earlier rejection files, 96 next
rejection files, and 135 files in the newly imported rejection are byte-exact.
Additionally, all 10347 parity-review/perturbation artifacts tracked at the exact
base were compared by bytes against Git. Earlier rejected review records,
including `0f00130a`, remain unchanged. No self-authored review.json is present.

Pinned Java source: `71806a23780b04a5ccde2f8ceb5121edad5eb711`.
JAR SHA256: `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
Fresh reference command:
`/Users/marcelo/rustuml-reference/bin/reference --cwd <worker>/new --timezone GMT+11:00 -- -pipe -tsvg`.
Worker: `/Users/marcelo/rustuml-reference/scratch/worker.w99EHYme`.
Java argv is derived from the pinned wrapper, **not process-traced**. No global
historical font/Graphviz environment equivalence is claimed.

Original immutable bundle SHA256:
`958158ca630045f0dfb3f917a67c2e2319699c1e701097d6861631b2bd420714`.
Returned gzip SHA256:
`3db82bcd6e971d1b52340d8104271bce28059cea3493f46b35eabb6f779a3cda`.
The parent's lossless files.base64 → resources.data mapping, exact embedded
metadata bytes, both validation records, raw streams, transport and original
request/manifest/bundle remain in the archive. `relay-envelope-hashes.json` and
`reference-validation.json` record independent checks.

`measurements.tar.gz` retains every membership, frozen source/reference, raw
before/after CLI SVG/YAML/comparison, command result, fresh relay stream, script
and log. Rebuildable binaries, target trees and bytecode caches are excluded.
`archive-files.json` records each member hash; `archive-integrity.json` records
archive read-back verification. `build-provenance.json` records binary/source
hashes, exact commands and revisions. Historical resource contracts and full
prior Java streams stay in their unchanged earlier archives.

Extract to a private cache; rebuild the recorded revisions with private targets
and jobs 4. Run `uv run --no-project --with pyyaml python run_replay.py before`
(or `after`) with explicit cohorts `retained maker18 sprites50 checker38 fresh15
fresh26 new15 delimiters4 focused2 checker30 supplement8 fresh12`. Restore or
losslessly map captured absolute paths and honor each cwd/include/env contract.
`validate_fresh.py` checks returned captures without executing Java. These are
maker measurements for a new independent review, not a declaration of acceptance.
