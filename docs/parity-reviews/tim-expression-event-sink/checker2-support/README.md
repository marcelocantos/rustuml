# Independent corrected TIM review

The complete packet is **rejected**. Ten new checker records (five numbered
successors and five first reviews) preserve the previous verdicts and failures.
This is a fresh review by `t3-2fa74908-independent-corrected-TIM-checker`, using
inherited `gpt-6-astra`, high reasoning, through Codex intentionally. No children,
production fixes, baseline changes, comparator changes, target changes or remote
operations were made.

## Exact tree and reference

Parent: `29642ec630e9902c892162a7081cd8a5e1113dac`.
The eleven requested cherry-picks, in the exact supplied order, produced
`86e0d0953acc1bbc660304265033c3314fa31150`, corresponding to maker tip
`b384bc315f31b890abb3cbd91197b6f6666ca1ad`. There were no conflicts or duplicate
account/rejection/registry imports. Parent account aliases remained byte-for-byte
unchanged. No class or sequence production changes were imported.

During this review the owner explicitly selected raw Java CLI/file semantics.
Policy `b62d8ce967b6dda7d85bb6dc28ff92509f6066f9` and assertion
`136ee057e0ffc3347ecdfafc9cf46e0bafa50197` were then applied, producing final
tested candidate `53f2e605c222408ee81c23ff5ea81c8bf7449084`.
`cherry-pick-mappings.json` maps all thirteen originals to local commits.
These last two commits change policy and the exact existing test's expected
origins, not production, references or comparator semantics. The root's later
class evidence import and its 1,192 cohort are not part of this checkout.

Java is pinned source `71806a23780b04a5ccde2f8ceb5121edad5eb711`, with source-pinned
JAR SHA256 `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
The absolute Temurin 21 executable, headless AWT and explicit `GMT+11:00` are
used for canonical new raw-pipe references. Named-zone and fractional-offset
cases record their own Java zone and Rust environment. The JAR is not claimed
historically byte-identical. Existing font/Graphviz equivalence is not assumed.

The first parent gate failed setup because the worktree's golden submodule was
empty. Ordinary initialization rejected the local file transport; scoped
`git -c protocol.file.allow=always submodule update --init test-diagrams/golden`
initialized revision `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`. A new complete
parent main-tier/guard run followed. The initial log remains archived and is
not a parity measurement. The initial perturbation run was valid because that
separate 955-pair corpus was already present.

All Cargo commands use `/tmp/rustuml-tim-check-2fa74908/target` and
`CARGO_BUILD_JOBS=4`. Stdlib is the pinned read-only checkout documented in
`docs/reference-bootstrap.md`. Each isolated measurement invokes the actual
release CLI in a separate process. The comparator probe calls unchanged
`compare_svg_strict` and unchanged Java-error detection. Source diagnostics
use actual Java `SourceStringReader`/`Diagram.getSource()` and shipped CLI
`--yaml`; they are supplemental, never substitutes for SVG comparisons.

## New causal counterexamples

* **Return function versus procedure events:**
  `checker2_return_plain_nested` renders only `Actual` in Java but also a
  `Phantom` class in Rust. The nested tick executes once in each.
  `TContext.executeOneLineNotSafe` uses `simulatePlain` for RETURN_FUNCTION,
  evaluating substitutions without adding the plain line; procedures use
  `addPlain`. Rust `FunctionDef` has no function-kind field, and `call_function`
  exposes every body event. The event-sink correction introduced a real lost
  pass on this fresh case compared with the exact parent.
* **Operand evaluation:** `checker2_logical_effects` gives Java `calls 1`, Rust
  `calls 0`. Java `ReversePolishInterpretor` evaluates operand tokens before
  applying the logical operator. Rust `eval_bool` short-circuits with host
  `||`/`&&`. The corresponding invalid date operand in
  `checker2_short_circuit_error` is rejected by Java (200) but Rust emits SVG
  and exits 0. This predates the correction but still falsifies complete
  execution/error parity.
* **Quoted values re-executed:** `checker2_literal_re_evaluation` renders
  Java `calls 0`, Rust `calls 1`. `eval_bool` recursively interprets a returned
  `Value::Str` as new expression source. Java treats the quoted token as a
  value. The separate literal function-text concatenation control passes.
* **Typed memory/default/lookup values:** literal default `"007"`, captured
  global `"007"`, and `%get_variable_value` all yield Java `0072` after `+2`;
  Rust gives `9`. Java `Eater.eatDeclareFunction` and
  `TFunctionImpl.getNewMemory` retain `TValue`. Rust stringifies values into
  `LocalMemory` and reconstructs them with `Value::from_str_auto`; builtin
  returns suffer the same loss. Declaration-time capture and default side
  effect order themselves pass. Named `$b="named"` also binds as raw
  positional text instead of Java's named argument overriding the second
  default. `TFunctionImpl.getNewMemory` is the missing binding model.
* **Valid signed arithmetic rejected:** `checker2_sum_wrap` computes
  `2147483647 + 1`, and `checker2_difference_wrap` computes
  `(-2147483647 - 1) - 1`, before `%date`. Both are valid Java and produce
  1901/2038 dates. `TValue.plus/minus` use Java int arithmetic; Rust uses
  wider floating arithmetic and then rejects the result as out of i32 range.
  These are two new valid-input CLI errors relative to the exact parent,
  although the parent also rendered wrong dates. They are not invalid literal
  overflow controls and are not excluded from valid-positive denominators.
* **Zone run widths:** `checker2_long_zone` gives Java
  `AEST / Australian Eastern Standard Time / +1000 / +10:00`;
  Rust repeats `AEST`. JDK21 `SimpleDateFormat.subFormat` chooses SHORT/LONG
  for `z` at width four; Rust ignores the run width. Other signed-instant,
  width, quoted-apostrophe, Unicode literal and fractional-offset cases pass.
* **External subblock selection:** Java-valid `checker2_sub_ignored_prelude`
  includes `Selected` then `After`; Rust drops `Selected`. Java
  `preproc.Sub.fromFile` selects raw located sub-lines before executing them.
  Rust `try_includesub` executes the whole file in a fresh context; the
  unselected prelude's invalid date stops that context before it discovers
  the sub, and `temp_ctx.error` is discarded. The selected-error variant
  correctly fails the real CLI.
* **Error replacement:** `checker2_two_errors` produces Java's first
  `Bad date pattern` failure; Rust reports the later `Date seconds outside
  signed 32-bit range`. Statement-level output suppression does not stop
  recursive expression evaluation or preserve the first error.

The new opposite-quote Unicode case has exact source but unequal SVG. The
nested theme/comment case has exact source but the repeated palette fails.
Both remain failures. Prior repeated palettes, empty themed class canvas,
standalone note, handwritten, activity style and sequence continuation failures
also remain failures. Two existing sequence panics persist; registry cleanup
prevents cascades. No source-oriented success is a waiver for these dependencies.

No new golden reads, fixture-label conditions, coordinate surgery, or changed
comparator/skip rules were found in production. Reviewed calendar/hash constants
have named Java/Gregorian provenance. The discovered failures concern model
semantics, not numbers to tune. Sawmill's single-file parse failed because it
looked for `.sawmill/scopes.yaml` below a file; direct source inspection was used.

## Typed parser handoff: bounded positive evidence and limits

Both new `error.take()` checks in `parse_with_base` return `ParseError` before
using partial output. The four original root controls and new pattern/overflow
controls inside functions, defaults, includes and selected subblocks fail the
actual CLI with no SVG. Errors in an unused function body stay inactive; a plain
inactive error assignment stays inactive. Java and Rust both evaluate declaration
defaults even inside the tested inactive branch because declaration handling
precedes the conditional iterator. That case was initially proposed as an
inactive positive, but actual Java rejects it; its classification follows the
reference, not the proposed label.

All deterministic fatal examples fail during the first preprocessing call.
The second call was inspected, including its immediate error check, but no
second-pass-only dynamic error was demonstrated. The two entry points differ
in teoz pragma preservation. No claim of dynamic branch coverage is made.

`error_probe.rs` records public typed API diagnostics separately. The convenience
`preprocess_full` uses caller-origin marking for a function error (line5), while
`parse_with_base`/CLI use the function body origin (line2), matching Java. The
legacy Vec-returning convenience APIs still discard typed errors by design;
they were not changed or advertised as checked APIs.

Default Rust CLI selects block0, whereas Java `-pipe` processes all blocks.
The new two-block negative therefore has a deliberately distinct default-stream
comparison. Explicit `rustuml --block=1 -pipe` rejects the failing second block
with exit1 and zero stdout. The default block0 success is not called a new
parser error-propagation defect. The selected-block experiment is also not a
second preprocessing pass.

The narrow parser handoff is positively corroborated. The complete error
mechanism remains rejected because evaluator failures above still reach the
user, and its first diagnostic is not preserved. Do not repair the two correct
handoff checks to compensate for missing upstream semantics.

## Owner-selected reader contract verified

The old review's statement that owner choice was pending is historical. Policy
`b62d8ce9` now selects raw Java pipe/file. The earlier reader-trace evidence is
reused: historical GET passes through `DiagramSourceCompressor.decompress`,
which removes outer body blanks before TIM; raw file/pipe/POST do not.

Three fresh whitespace/delimiter cases (tight LF, leading/internal/trailing CRLF,
and named delimiter with trailing blanks) pass raw Java pipe, real Java file,
and real Rust CLI comparisons. Three additional raw counterparts preserve the
unchanged source bytes of `act_skinparam2_shadowing_true`,
`act_skinparam_shadowing`, and `edge_whitespace_multiple_blank_lines`; all pass
both file and pipe comparisons. Their references are new explicitly raw pairs;
no frozen GET reference was rewritten and no historical pass is inferred.

The exact unchanged assertion source independently yields Java/Rust origins
5/8/9; `raw-assertion-origins.json` records the attributes. `file-reader-results.json`
records all six file checks. One checker setup attempt expected the source-stem
filename for the named delimiter; Java correctly wrote `canopy.svg`. The initial
error and corrected comparison are preserved; no product rerender was needed.

## Measured results and denominator discipline

The frozen parent inventory is exactly **955** source/reference pairs, including
six registry controls missing from the maker's 949. Independent root-cwd actual
CLI runs give parent **379 matches**, candidate **431 matches**, with **52 gains,
zero lost passes**. Every gain is listed in `isolated-delta.json`. Candidate
retains 512 valid mismatches, two existing sequence panics, nine Java error SVGs
and one empty Java reference. The two new process errors in this cohort are the
invalid date controls now correctly rejected. Do not count those as valid matches.
The initially reported `invalid_reference=12` for the parent incorrectly included
two malformed Rust panic outputs; the retained rows now distinguish malformed
expected XML from malformed actual output, yielding the correct ten invalid
references. Comparator semantics did not change.

Built-in package-cwd replay gives parent **378/955**, candidate **430/955**.
The single difference from isolated root cwd is the previously known
`checker_include_theme` include-path failure. The original five maker-owned
rendering failures remain unwaived. The previously reported maker-relative
loss `checker_selector_nested_theme_event` still fails; it was not a pass on
this review's exact prepared parent, so it is not falsely listed as a new
parent-to-candidate lost pass.

Existing source replay confirms **40/40 valid prior TIM checker sources** and
**33/33 original theme sources** (including its one invalid Java reference).
Those cohorts produce 31/40 and 28/32 valid SVG matches respectively.
The retained maker set has **42 inputs**: previous22, corrective16, root4.
There are **34 valid inputs, 34 exact sources, 33 SVG matches**; eight invalid
controls remain separately classified. The corrective maker's 14 valid cases
all pass. Prior retained activity remains failed.

This checker adds **89 pairs**: 46 fresh diagram/counterpart cases, 42 retained
maker copies, and one custom-theme helper. The helper is deliberately separately
identified: CLI SVG matches, but raw `SourceStringReader` has no standalone block
for its headerless text, so its failed source probe is not a TIM stream claim.
Of the 46 fresh diagram/counterpart cases, **32 are Java-valid**, **19 match SVG**,
and **21 match evaluated source**. Fourteen are Java-negative, of which twelve
are rejected by default Rust CLI without output; one is the short-circuit bug,
and one is the explicitly selected multiblock case discussed above. These are
not 89 independent inventions or denominator replacements.

The total local corpus is **1044 = frozen955 + new89**. Isolated real-CLI union
has **484 SVG matches**, **32 invalid references**, and **24 nonzero CLI exits**
(overlapping negative-reference controls; two are genuine sequence panics).
Fresh return-plain is a lost SVG pass relative to the exact parent on new inputs;
fresh arithmetic wrap adds two valid CLI errors. Existing no-oracle held-out
checks automatically discover all new cases; no gate/exclusion was altered.

Independent main release tiers on the exact parent: product **11181/11251**;
assisted **10497 pass, 754 fail, 1299 skipped**. Candidate product is
**11179/11251**, assisted unchanged. The historical product loses exactly the
three GET-vs-raw pairs above and gains `preprocessing/preproc_complex_theme_override`.
Those historical losses remain reported even after the raw policy choice.
Ratchets remain red and unchanged. Full final logs/counts are in
`verification-summary.json`; no isolated source result replaces these tiers.

Parent parser units: **561/561**. Final policy/assertion candidate: **577/577**.
CLI: parent **13/13**, candidate **15/15**. Both prepared trees have **4/6 guards**;
decimal and existing rejected review evidence remain red. Workspace formatting
fails on pre-existing class files; all five touched Rust files pass rustfmt.
Clippy fails on the unchanged unused
`parse/component.rs::looks_like_description_bracket_command`; no cleanup was
performed. The new review records are rejected, so a red evidence guard remains
correct, not a schema excuse to accept this packet.

## Class handoff and bounded repair guidance

Class review `502960ac950d62a1e3a5e57f7f01664b694f146b` names five TIM inputs with
new strict-parser errors. All five are already in frozen955. This candidate
independently matches Java **source and SVG on all five**: comments/origins,
explicit global override, global-over-auto-local, dump_memory and inner comment.
`class-tim-handoff.json` links the exact existing measurements. No class production
was imported, so the integrator must still test the combined strict class parser;
this is not a claim that its complete packet passes.

Repair return/procedure kind and Java operand/value semantics first. Retain typed
values across defaults, memory, builtin results and named binding. Model Java
int arithmetic before date conversion, and width-sensitive timezone display.
Select external sub-lines before executing them, and preserve the first fatal
error through expression recursion. Keep the verified raw-reader contract and
parser boundary checks. Recheck all retained failures and new cases with a fresh
independent checker; class/sequence/shared rendering dependencies remain real.

## Reproduction and preserved evidence

`final-manifest.json` identifies every added pair; `parent-inventory.json` hashes
all original955 inputs/references. `candidate-additions.json`, `parent-additions.json`,
`candidate-isolated.json`, `parent-isolated.json`, source replay, raw reader and
class handoff files retain individual outcomes. Original reviews/assertions
(except the explicitly authorized assertion cherry-pick), references, baselines,
park lists and comparator are unchanged.

`checker.py`, `diagnostics.py`, `final_controls.py`, generation scripts, Rust probe
and the command ledger are support only. Build this tree with the environment
above; compile the inherited comparator probe against the matching release
rlibs and `ReaderProbe.java` against the pinned JAR into the scratch directory.
Use `checker.py candidate fresh` with `final-manifest.json` for added pairs,
`checker.py candidate isolated` for frozen955, and `diagnostics.py` for retained
source traces. Java SVGs in the new corpus were generated through the real CLI;
source probes did not fabricate references.

`measurement-artifacts.tar.gz` preserves raw outputs, errors, comparison details,
initial setup attempts and full gate logs. `artifact-hashes.json` and the command
ledger pin these artifacts and tools. Captured raw source/diff whitespace is
intentional evidence; it is not normalized to silence `git diff --check`.

The built-in final perturbation gate also has a declared environment limitation:
its fixed `GOLDEN_DEBUG` cannot replay the two explicitly noncanonical zone
settings in `final-manifest.json`. The named-host long-zone failure is proved
through the matching host-zone CLI path; the fractional-offset control passes
that path and its built-in fixed-offset mismatch is not labeled a new product
failure. Both generated pairs and their exact environments remain preserved,
and the environment-aware checker script remains executable. No acceptance or
exclusion rule was changed to hide the difference.

Final built-in result is **480/1044**, with four differences from the matching-
environment root-cwd CLI union: the original `checker_include_theme`, retained
`local_theme_once`, new custom `theme_from_order` (cwd-dependent resources), and
`fractional_zone` (fixed gate offset versus declared supplemental offset).
The final guard rerun has **4 pass, 2 fail** and no schema/path errors in these
new reviews. An initial checker-record draft put cross-mechanism dependencies
inside the owned `perturbations` arrays; the guard caught that. Those links now
live in explicit `dependency_perturbations` fields, with each fixture retained
under its owning record. No duplicate fixtures, verdicts or guard rules were
changed to address this metadata error. The initial and corrected logs remain.
