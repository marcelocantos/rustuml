# Independent typed TIM execution check

**The packet is rejected.** Fresh source-guided cases expose one lost SVG pass,
14 valid fresh SVG failures, and missing fatal errors. The repaired historical
cases do improve. None of the known renderer dependencies is waived because its
evaluated source agrees.

Reviewer: `t3-bf706fe0-fresh-independent-typed-TIM-checker`, **gpt-6-astra / high**,
Codex through T3 Code. This task did not implement the candidate, ran no children,
and changed only new review records, support evidence and held-out inputs. All
Java ran remotely on Monkey Man; no local JVM or javac ran. No production fixes,
target updates, reference rewrites, comparator/baseline changes, pushes, PRs or
releases were performed.

## Exact preparation and oracle

Parent: `caae198aabd7a20a989e906b42749602a1d73cc5`.
The eight requested pre-continuation commits produce before revision
`ed0dcd3c9c11de50288f914947e6735005fb6bb5`.
The subsequent accounts, production, evidence and root test-only follow-up produce
final tested revision `579b9b1173fef9dd3d10afca05b6d688b9bbd2c2`.
This corresponds to original production `0dd2de894fea40d5e9ed81efb6a54d837f80fa4c`,
maker evidence `f9f88b766a2b5c8cbabe90c4a6811f1a6f6fc306`, and root assertion
`6f52f87e532d549b047f7ccd3fe7fb577fc56df9`.

`preparation.json` records all twelve original/prepared mappings. Cherry-picks
were clean; b384bc31's imported root evidence already matched the parent's exact
bytes, while its production and test hunks applied. No class or sequence candidate
production was imported. `preservation.json` verifies exact requested TIM code,
unchanged parent review files and frozen inputs/rules, real golden submodule
`9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`, and binary/comparator hashes.

Class-2 reference comparison is adjudicated by the **real release CLI** and
unchanged `compare_svg_strict`; source/model/diagnostic probes are supplementary.
Every fresh reference uses exact input bytes through the remote raw `-pipe -tsvg`
front end. Java source is `71806a23780b04a5ccde2f8ceb5121edad5eb711`; JAR SHA256 is
`aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.

Remote worker: `/Users/marcelo/rustuml-reference/scratch/worker.l4Yx4xIn` on
`marcelo@192.168.1.112`. The pinned wrapper forces headless, UTF-8 and en_AU.
Its OpenJDK 21.0.5 / Graphviz 12.2.0 differ from origin 21.0.13 / 16.1.0.
The root's seven calibration pairs establish only that scope. This check also
re-rendered eleven regression controls remotely and compared their preserved
references without rewriting them. Matching per-case TZ and include cwd are
recorded; no fixed-gate environment mismatch is called a new product defect.

## Causal counterexamples

1. **Typed builtin adapters lose the type again.**
   `checker3_substr_json` is a **lost pass**: `%substr("cedar", %str2json("2"))`
   produces Java/before `cedar`, after `dar`. Java `Substr.executeReturnFunction`
   calls `TValue.toInt`, whose JSON and string integer field is zero. Rust
   `eval_builtin_with_expr_args` keeps Json(2), then maps all values to display
   strings before `eval_one_builtin_args` parses usize 2. The before pass was
   accidental: its unsupported str2json returned empty. That does not erase a
   shipped-path regression. `substr_string` and `hex_string` independently expose
   the same missing typed argument contract. `hex_negative` formats an i64 hex
   value rather than Java `Integer.toHexString` (16 digits versus 8).
2. **Operand tokens are read at the wrong time.**
   `checker3_snapshot` gives Java `14 / 5`, Rust `15 / 5`; the logical variant
   gives Java `0 / 1`, Rust `1 / 1`. Java `ShuntingYard` resolves PLAIN_TEXT through
   `Knowledge.getVariable` into value tokens before `ReversePolishInterpretor`
   executes the function calls. Rust `eval_expr_to_value` recursively reads its
   right operand after executing the left call. Eagerness alone is insufficient.
3. **Returns do not unwind loop execution.**
   `loop_early_return` gives Java `1 / 1`, Rust `3 / 3`. `foreach_return` gives
   Java `0052 / 1`, Rust `0062 / 2`. Java `TContext.executeLines` returns immediately
   on a returned TValue. Rust `try_while` and `try_foreach` continue body/iteration
   processing after `return_signal`, mutate state and replace the first return.
   Separately, `procedure_expression` is rejected by Java's function-kind check
   but Rust accepts it and emits an unexpected class.
4. **External selection still misses valid syntax.**
   `tabs` uses `!startsub` followed by a tab; Java's TLineType/EaterStartsub accepts
   it while Rust's literal-space prefix drops the selected block. `dynamic_selector`
   expands the selector through Java `EaterIncludesub.applyFunctionsAndVariables`;
   Rust matches its regex first and leaves the expanded directive as plain output.
   Both Java cases mutate the counter to 3; Rust leaves it at 0. Repeated/nested
   raw blocks and caller-relative nested includes do pass. The account's claim
   that all unselected contents cannot fail is overbroad: Java still parses every
   STARTSUB marker via EaterStartsub; selection suppresses unselected **body
   execution**, not all structural validation.
5. **The first failure is sometimes never represented.**
   Java `%substr("abc",-1)` throws before the subsequent date expression; Rust
   defaults its negative index to zero and reports the later Q date error.
   Unknown `%missing_checker3()` similarly becomes empty in Rust, replacing
   Java's Unknown built-in function outcome with a later X error. Isolated variants
   without the later error cause Rust exit 0 and SVG containing `After` while
   Java fails. Default-value and compound-expression controls preserve the
   exceptions and stderr. This is missing error production/continued execution,
   not overwriting an already-populated `ctx.error`.
6. **Other typed/text routes remain incomplete.**
   `json_index` lacks Java `TContext.asKnowledge/fromJson` member/index resolution;
   Rust emits the object plus `.entry[0]2` instead of `0092`. `literal_macro`
   re-expands a returned/assigned literal `NEXT` into `changed` through the legacy
   token loop. `unicode_index` uses UTF-8 byte indexing (6 versus Java UTF-16 3);
   `unicode_slice` uses Unicode scalar indexing (`b` versus `a`). These are source
   and SVG failures, not renderer-only guesses.

The whole changed preprocessor module and date/debug modules were inspected,
including typed memory, named/default bindings, builtin result dispatch, legacy
adapters, execution-kind save/restore, conditions, recursive calls, loops,
selected includes and fatal guards. The named Java implementations above,
TFunctionImpl.getNewMemory, TValue, token/RPN flow, TMemoryLocal, Sub.fromFile and
TContext execution were inspected at the pinned revision. No new golden reads,
fixture-label predicates, coordinate surgery or unexplained parity constants
were found. The timezone table is independently derived locale data, not diagram
replay. The failures identify missing model behavior rather than numbers to tune.

## Supported local invariants and limits

Recursive return functions now suppress ordinary body text while executing
nested effects; procedures and includes retain their own emitting execution
kind. Captured typed defaults, named overrides and sibling/global memory controls
pass. Recursive AND/OR evaluates both operands; quoted Unicode function-like
values are not executed. A first **recorded** date failure survives later calls,
and the two parser error handoffs reject partial output.

Signed i32 sum/difference/multiplication wrap and truncating division, including
INT_MIN/-1, are corroborated before date conversion. Java-invalid shifts and
unary variable forms remain negative scope probes. The documented no-return
function extension remains Java-negative, distinct from illegal procedure use
in an expression. Existing floating extensions were inspected, not promoted to
full Java grammar parity or newly comprehensively tested.

**Zone width selection has no fresh semantic falsifier here.** Eight fresh
cases and three retained maker cases match full source and SVG under matching
zones: London standard/daylight, Lord Howe, Dublin, Kathmandu, Casablanca
Ramadan/summer, UTC and fixed fractional offset. A remote rerun of ZoneNames.java
reproduces all 631 IDs byte-for-byte (SHA256
`1b99a8affbdcaba1600e0f18d39126e68a461b1c3040ffc9eed29606d7f87483`). SHORT/LONG
selection and its provenance are supported on these paths. Its review is not an
acceptance certificate for this red packet: the required standing perturbation
gate cannot replay these declared environments with its fixed override, and this
review cannot change that contract. This is explicitly a gating/provenance
residue, **not a newly demonstrated zone defect**. Other OS/libc/runtime/locale
and unknown-zone fallback behavior remain unproved.

Root assertion follow-up is independently corroborated. All four exact unchanged
class/sequence inputs are retained. Remote Java's literal overflow stack is
`Integer.parseInt -> TValue.fromNumber -> ReversePolishInterpretor`, with no
DateFunction frame. Rust reports its generic integer-literal error at line 2;
the original expect-error and exact-line assertions remain intact and pass.
The diagnostic strings are not claimed byte-identical: Java's numeric exception
escapes rather than emitting the Rust-specific wording. Source/error/line checks
were not weakened.

`sub_caller_relative` remains a valid SVG failure despite exact source. The Rust
model retains `Child.source_line=0`; the class renderer's zero fallback substitutes
entity index + 1, and SVG entity emission ordering differs. Repeated palettes,
opposite-quote aliases, empty themed class canvas, notes, handwritten, activity
and sequence dependencies remain failed and unwaived. No combined class/sequence
candidate was tested. Second-preprocess-pass-only dynamic error coverage remains
undemonstrated. Legacy Vec-returning convenience preprocess APIs still discard
errors; only the checked parse path is claimed. The default block0/all-block
reference distinction is preserved, not reclassified as an error-handoff defect.

## Measurements and denominator discipline

`frozen-inventory.json` hashes the parent inputs and references; no executor-owned
acceptance denominator was substituted. Actual root-cwd CLI results are:

| Cohort | Before matches | After matches | Lost passes | New valid errors / panics |
| --- | ---: | ---: | ---: | ---: |
| Frozen955 | 431 | 431 | 0 | 0 / 0 |
| Frozen1044 = 955 + 89 | 484 | 495 | 0 | 0 / 0 |
| Parent1449 collateral replay | 509 | 520 | 0 | 0 / 0 |

The 1044 cohort retains 32 invalid references and two sequence panics; the two
valid arithmetic CLI errors are repaired. Its one additional correct rejection
is the prior short-circuit negative. Parent1449 has 38 invalid references and 32
pre-existing panicking entries, including repeated sequence enforcement inputs;
these are not 32 new independent bugs or registry cascades. Candidate valid
nonzero exits number 33 there, compared with 35 before. The missing six registry
controls explain the maker's 1038/949 denominators; they are present and passing
here. No unlike-count subtraction is used.

This checker preserves **67 new pairs**: 46 fresh cases (42 initial + 4 targeted
follow-ups), 17 exact maker source copies re-rendered remotely, and four exact
root assertion sources. The fresh set has 34 valid references: 20 match SVG and
source, 14 fail both. Before it has 2 matches; after has 19 gains and **one lost
pass** (`substr_json`). No new valid CLI errors or panics occur. The 17 retained
maker cases reproduce 14 valid/exact-source cases, 13 matching SVGs and three
negatives. All four root assertion copies are negative. These retained copies
are enforcement entries, not independent discoveries or positive progress.

There are 19 new negative references in total. Rust rejects 13 without SVG and
accepts six: documented no-return extension, illegal procedure expression,
shift/unary grammar probes, unknown builtin and invalid substr. Invalid or empty
Java output stays negative in the unchanged oracle; successful Rust rejection
does not turn it into a positive SVG match.

All five class TIM controls and all six raw reader controls pass source and SVG;
all eleven new remote renders also match their preserved Java SVGs. The raw
reader policy is confirmed as a regression control, not reopened.

Final main release tiers were each run **once** on the exact prepared candidate:
product **11179/11251**; assisted **10497 pass, 754 fail, 1299 skipped**. Both gates
remain red. The three approved raw-versus-historical-GET losses remain reported
without rewriting historical references: two activity shadowing inputs and the
multiple-blank-lines edge case. No fresh full main-tier before run is claimed.

Parser **582/582**, CLI pipe **15/15**, binary unit target **0 tests**. Touched-file
rustfmt passes. Clippy fails on the unchanged unused
`parse/component.rs:72::looks_like_description_bracket_command`. Guards remain
**4 pass / 2 fail** (decimal ratchet and review evidence). New owned review records
have no schema/path/missing-pair errors. The candidate's unmodified maker-support
directory `tim-corrective-execution-maker` also lacks the account.json demanded
by the existing directory-walking guard; this remains a maker metadata residue,
not a reason to alter guard rules or invent a checker-owned maker account.

The unchanged built-in perturbation gate reports **532/1516**, 984 failures,
including 32 pre-existing panicking entries. Matching-environment actual CLI union
is **553/1516**. The exact 21 differing paths are listed in `builtin-results.json`:
include cwd and named/fractional-zone overrides explain these differences. They
are not new product losses. The suite is red; no claim of acceptance or a green
whole-product gate is made.

## Bounded next repair and reproduction

Keep the demonstrated typed-memory and event improvements. Replace string
adapters with each Java builtin's typed argument contract; preserve UTF-16 index
semantics; resolve variable tokens before RPN execution; implement JSON access
and literal expansion boundaries; propagate return through loop execution;
enforce procedure/value call kinds; evaluate includesub selectors and recognize
actual directive whitespace; produce fatal errors before relying on cancellation.
Do not modify correct parser handoffs or compensate with renderer offsets.

Fifteen new review records preserve all earlier rejections: five first reviews
for the new accounts and ten numbered reassessments. The broad packet remains
rejected. Narrow supported observations above are not extraction permission for
untested production hunks. No review history was overwritten.

`check.py` invokes the actual CLI and unchanged comparator. `generate.py`,
`supplement.py` and `retain.py` record the input matrix; every generated source
and same-stem Java SVG is under its owning mechanism. `.inc` resources preserve
relative hierarchy. `final-manifest.json` declares input environment and status;
`reference-*.json`, before/after result rows and `deltas.json` retain all outcomes.
`reference-environment.json`, `preparation.json` and `preservation.json` pin
provenance. `measurement-artifacts.tar.gz` preserves stdout/stderr, SVG differences,
source/model probes, all 1449 before/after captures and full test logs. Raw tool
captures were first written to private `/tmp`, then deliberately archived here.

Use private Cargo target `/tmp/rustuml-tim-checker3.dRQvKH/target`, at most four
build jobs, and the pinned PLANTUML_STDLIB from reference-bootstrap.md. Build the
before revision and copy its CLI to `before-rustuml`, then the final revision to
`after-rustuml`. Compile `probe.rs` with rustc against those local release rlibs;
its compare branch calls the unchanged library comparator. Run
`uv run --with pyyaml python .../checker3-support/check.py before frozen` and
`after frozen`. For new cohorts use `fresh`, `retained` and `supplement`; reference
mode requires allocating a new private remote worker, staging the same relative
hierarchy and SourceProbe.java with SCP, and setting `TIM_CHECK_REMOTE`.
`TIM_CHECK_SCRATCH` selects private local captures. Java probe and SVG command
arguments are recorded; no origin-side JVM is necessary. Gates and exact commands
are recorded in the reviews and archived logs. The harness is a reproduction aid,
not a replacement acceptance gate or a reason to call the red suite green.
