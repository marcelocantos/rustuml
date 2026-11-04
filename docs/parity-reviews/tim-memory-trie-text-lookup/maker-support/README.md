# TIM checker7 corrective maker packet — acceptance pending

This is implementation evidence, not an independent review. Every prior account,
review, verdict, source and reference remains unchanged. A new independent checker
must assess these three bounded corrections. No acceptance is claimed, and the
parent has not integrated this TIM production.

- Start: `2cf6c7eeebae7354e7a3b7e53b86fb3754883e67`.
- Independent checker7: `a7b3be50308f93ab2d97d6aee5fb9fbda522b7f2`, imported as
  `95c114314f479be301ff9549f4f1d3edf2616a28`.
- Three causal accounts committed before edits:
  `02fa49de2396c1445a4ddf1a783c973f883a82f4`.
- Implementation: `2274e808433568aa1d8042bf9e3cb1f1de338b4b`.
- Maker: inherited gpt-6-astra/high, Codex through T3, isolated checkout
  `/Users/marcelo/.t3/worktrees/rustuml/t3-2c674563`, no children.

## Corrections and source derivation

`tim-memory-trie-text-lookup` couples every memory insertion/removal to its name
index. Text lookup follows the stored names, including internal dollars,
combining characters, punctuation and spaces; it does not invent an identifier
character list. The index follows Java `TrieImpl` UTF-16 edges. It returns only
the terminal at the traversal's stopping node, without falling back to an earlier
terminal. Removing a name retains prefix nodes. Those details distinguish it from
ordinary longest-stored-prefix matching. Global and automatic-local indexes are
eager; argument/explicit-override indexes are built lazily as in `TMemoryLocal`.
The longest returned match across scopes is selected before the right boundary
check; value lookup retains override/global/local precedence.

`VariableManager.getVarnameAt` supplies the original-text boundaries, including
literal backslash-n and dollar exceptions. Its boundary predicate is Java's
letter/ASCII-digit/underscore/surrogate rule, not the admitted name grammar.
Variable concatenation markers are consumed around matched substitutions. The
original text is scanned once, with function priority; appended values are never
rescanned. The old sprite-markup shortcut is removed because
`TContext.applyFunctionsAndVariables` scans those original positions too.
`TrieImpl.add` rejects embedded NUL; builtin insertion uses the existing fatal
error path. General isolated-surrogate projection and Unicode-version table
identity remain outside any unqualified text-equivalence claim; the BMP letter
property uses the existing Rust regex Unicode data, not a newly certified JDK
character database.

`tim-duplicate-formal-binding` changes each formal to a non-consuming typed read
from the named map, as in `TFunctionImpl.getNewMemory`. Only the positional branch
advances its cursor. Captured defaults remain values, and the final assignment
wins for duplicate exact formal keys. Signature selection, arity checks and RPN
named-map lifetime are unchanged.

`tim-empty-sub-iterator-termination` models the empty replay installed by
`CodeIteratorSub.peek`. Since peek returns null before next can clear the replay,
the current `executeLines` iterator stops. Tail content, effects and the end tag
are not emitted. Nested includes/sub execution receive their own iterator lifetime
and restore the caller when they finish. Nonempty replay stays in the original
iterator; comments, collectors, loop/return behavior and source origins retain
their existing ownership. Both source and parser projections consume the same
truncated events. No parser, renderer or raw-reader policy was changed.

Seventeen pinned Java files were verified byte-for-byte against revision
`71806a23780b04a5ccde2f8ceb5121edad5eb711`; hashes are in
`java-source-provenance.json`. Production changes only `preprocess/mod.rs`.
All **129 original preprocessing test blocks are unchanged verbatim**; seven
new tests exercise trie history/scopes, original scanning, duplicate binding and
iterator lifetime. `unchanged-units.json` records that check.

## Actual CLI and source observations

All positive SVG counts use the shipped CLI and unchanged strict comparator.
Nonpositive references remain in membership and never gain positive credit.

| Membership | Entries | Valid refs | Before SVG | Final SVG | Final exact valid source |
| --- | ---: | ---: | ---: | ---: | ---: |
| All retained cohorts | 1,672 | 1,560 | 665 | 665 | 158/160 available |
| Checker7 fresh23 | 23 | 19 | 14 | 17 | 19/19 |
| New maker13 | 13 | 11 | 3 | 10 | 11/11 |
| New supplement2 | 2 | 2 | 0 | 1 | 2/2 |

The retained cohorts still include every entry of restored1516, prior31,
checker5 fresh29, maker15/17/18, checker6 fresh29, TIM7 maker15 and supplement2.
Separate summary JSONs retain their denominators: restored1516 stays 567 matches;
955/1044/1449 subsets stay 431/495/520. No retained valid pass was lost, and no new
valid CLI error or valid entity-count drop was observed. Checker7's three TIM
failures now match source and SVG. Its two downstream class failures remain.

All memberships total **1,710 entries, 1,631 distinct source/reference pairs**.
All 79 duplicate entries are within restored1516, and there are no intersections
between the other cohorts. `analysis.json` lists every intersection. This is this
checkout's cohort, not the parent's larger integration inventory.

The retained 32 sequence panic entries remain (24 len-2 and eight len-1 at
`sequence.rs:8022`). The class `body_ownership` parse error and two Unicode IntVal
CLI errors remain. The two IntVal cases are the retained exact-source failures.
All six raw-file controls and five retained class/TIM controls pass; this is not
combined class integration evidence.

Checker7's empty-sub cases now match the exact Java source
`@startuml\nclass Anchor\n`. Their Rust class model loses the unexecuted Tail,
as it should, but still differs from Java's welcome factory. These two model-count
drops are nonpositive outcomes, recorded separately from valid model regressions.
The two new empty/comment-only collector cases have the same source behavior.
No welcome output is counted as an SVG pass.

## New reference classification and downstream handoffs

Every new Java SVG and source probe exits 0, every SVG is well-formed XML, and
none timed out. Those process facts do not establish validity. Original13 has
11 valid CLASS references and **two welcome outputs after truncation**.
Supplement2 has two valid CLASS references. `new-reference-classification.json`
records observed factories, welcome text, source/end-tag presence through the
captured source, comparator rejection and process status separately from intended
validity. The legacy `negative` count in relay summaries means all nonpositive
references, not that welcome output is a syntax error.

Three measured dependencies remain outside preprocessing:

1. **Welcome factory selection.** Java `TimLoader.load` retains the stopped
   `TContext.resultList`; `BlockUml` passes it to `PSystemBuilder`, where
   `PSystemWelcomeFactory.createSystem` chooses welcome when evaluated
   `UmlSource.getTotalLineCount()==2`. Rust `parse/mod.rs:1153` detects subtype
   from projected content and selects Class for Anchor. The exact truncated
   evaluated source is already available in `PreprocessOutput.uml_source`.
   The owner acknowledged this parse/factory and possible renderer handoff;
   manufacturing source/comments or changing reader normalization is not a fix.
2. **Source line zero.** New valid `empty_include` matches evaluated source and
   correctly resumes the caller after the included iterator stops. Its YAML
   Head entity has `source_line: 0`, matching the included Java origin. The only
   SVG difference is Java line 0 versus Rust line 1: `render/class.rs:1253` uses
   `entity_index + 1` when the parser's line is zero. The original source and
   reference remain unchanged and the case remains a product failure.
3. **Unresolved sprite text rendering.** Supplement `markup_scan` matches the
   exact source `class "<$label> / <space key> / <$missing>" as Markup`.
   The stored literal is not rescanned. Java renders unresolved sprite markers
   differently; Rust emits the whole literal as text, yielding different element
   count and dimensions. Both CLIs succeed, but SVG parity remains false.

Checker7 `select_0`/`select_1` remain unchanged failures with exact evaluated
source and truncated downstream class labels; the class owner has that handoff.
The prior disconnected class/notes packing mismatch also remains. None is waived.

## Final gates and iteration record

Broad required checks ran once after production stabilized and was committed.
The `iteration2` measurements use the byte-identical final executable; renaming
it to `final-rustuml` did not trigger a redundant corpus replay. `gates.json`
records commands, cwd, environment, duration and real exits:

- Parser release units: **622 passed**, exit 0.
- CLI pipe integration: **15 passed**, exit 0.
- Main product library diagnostic: **11,179/11,251**, exit 101.
- Main assisted library diagnostic: **10,497 passed, 754 failed,
  1,299 skipped of 12,550**, exit 101.
- Standing perturbations: **609/1,628**, exit 101; matching-environment isolated
  CLI **636/1,628**. The same 27 cwd/environment discrepancies remain enumerated.
- Guards: **4/6**, exit 101 (decimal ratchet and review-evidence failures).
- Clippy: exit 101 at unchanged `parse/component.rs:72` dead code.
- Touched rustfmt and production diff whitespace: exit 0.

The main tiers are library diagnostics, not CLI headline measurements. The
historical GET-versus-approved-raw reader conflict remains unchanged. No gate,
comparator, threshold, baseline, exclusion or policy was edited.

Focused iteration1 used the trie/binding/iterator correction before removing the
old markup skip; its fresh23 replay and module snapshot are retained. It passed
615 existing parser units. After adding seven tests, the focused preprocessing
run passed 136; the markup-specific test also passed after skip removal. No old
assertion was changed. All intermediate mismatches remain in the archive.
Two local analysis-script failures (a duplicate Python keyword, then a capture
subdirectory error) are retained in `analysis1.log` and `analysis2.log`; correcting
those scripts caused no product or oracle rerun.

Remaining boundaries include signature tree bins/JVM iteration and legacy
occupancy, Unicode IntVal, isolated-surrogate projection, function-in-index
syntax, downstream raw-sub-marker acceptance, error-page/line-number parity,
renderer origin fallback, class/style/palette, handwritten/activity and sequence
geometry, and the existing environment discrepancies. The successful date model
is untouched and is not a global environment equivalence claim.

## Transport, integrity and reproduction

No local Java/javac, worker SSH, child agent, network configuration or shipping
action occurred. The parent used restored SCP with roundtrip byte verification
and its existing Monkey Man runner. Both immutable requests use the same pinned
headless wrapper/JAR, raw pipe, UTF-8, en_AU and GMT+11:00. All 30 calls retain
actual wrapper/Java argv, cwd, environment, stdout/stderr hashes and status.
Remote Java 21.0.5 and Graphviz 12.2.0 differ from origin Java 21.0.13 / dot 16.1;
new captures establish only scoped agreement.

| Batch | Request SHA256 | Returned gzip SHA256 |
| --- | --- | --- |
| Maker13 | `1bed17bb10fbf5cec13d98cba7030659ce68485976727f64427664f58c23c041` | `ee90662329405f51a1904dd1b9bd5c093913d0e55d5e76162a398ea03c6f3ab4` |
| Supplement2 | `c16d701fccaa0f76345625cfc27709617d5cd6e09f9eb5de2be9631d4632a061` | `f1ef583736595411885dcc8cafcecb36d9b2cb36b0f0a602929cc1b49aef05ee` |

Manifest/resource/input-transport hashes and private worker paths are in each
request/environment/transport record. The original13 uses worker
`/Users/marcelo/rustuml-reference/scratch/worker.nN4V9LkS`. The pinned JAR is
`aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
`inputs/` and `supplement/` preserve all 15 sources and same-stem Java SVGs, plus
the exact include resource. `relay13/` and `relay2/` preserve raw bundles and all
decoded reference streams/statuses. Parent validation records are retained.

`integrity.json` verifies **9,245 prior evidence files** against imported Git
blobs with zero modifications. The golden gitlink remains
`9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`. Separate build targets are recorded:
the unchanged checker7 candidate target supplies the before binary; the maker
uses its own persistent target, with `CARGO_BUILD_JOBS=4`. No source checkout
switch or before rebuild contaminated the current target.

- Before binary SHA256: `43d564a4c6f5bd3b80469c158ba6840089be2ed3954996ee614295d323959608`.
- Final binary SHA256: `c44f7a80fbfb22cbd3f6fe008395694d6813b0f911b160c03783e1cdb80fd726`.
- Unchanged comparator SHA256: `43ce27167f6ad8697a8043edbb4f580ddbf9ab68b4e0ea4db15f2a69e3fc97b5`.

`measurements.tar.gz` preserves the private cache except executables, including
all local captures, failed iterations, scripts, memberships and content-addressed
source/reference bytes. `files.json` and `archive-integrity.json` verify packaging.
Before results for retained/fresh23 cases reuse checker7's exact-candidate `after`
records; the 15 new cases run the actual before CLI as well as current CLI.

To reproduce, restore the archive to
`/Users/marcelo/Library/Caches/rustuml-tim8-maker`. Restore `replay-inputs/` records
to the checker7 cache root if absent; `replay-input-paths.json` maps absent
source/reference paths to archived bytes. Keep relative include hierarchy and
pinned stdlib intact. Build revisions into separate targets, copy the candidate
executable to `iteration2-rustuml`, and restore/build the unchanged comparator
from `tim-typed-values-and-binding/checker3-support/probe.rs`.

```text
CARGO_TARGET_DIR=/Users/marcelo/Library/Caches/rustuml-tim7-maker/target-after CARGO_BUILD_JOBS=4 cargo build -p rustuml --release
python3 /Users/marcelo/Library/Caches/rustuml-tim8-maker/replay.py iteration2 collateral
python3 /Users/marcelo/Library/Caches/rustuml-tim8-maker/replay.py iteration2 fresh23
python3 /Users/marcelo/Library/Caches/rustuml-tim8-maker/relay_eval.py iteration2
python3 /Users/marcelo/Library/Caches/rustuml-tim8-maker/relay2_eval.py iteration2
```

The other replay suite names are prior31, fresh29, maker15, maker17, maker18,
fresh6, newmaker15 and supplement2. These scripts use existing references and
only local Rust executables. Do not regenerate any immutable source/reference
request. Final gate commands and red statuses already stand in the evidence.
