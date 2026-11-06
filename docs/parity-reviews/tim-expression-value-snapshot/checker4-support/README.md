# Independent TIM checker 4: rejected

The candidate is **rejected**. Fresh independent tests expose **one lost SVG pass,
one new valid-input CLI error, seven valid SVG/source failures, and missing fatal
errors**. Existing gains are real but do not establish packet acceptance.

Reviewer: `t3-cb14b5f3-fresh-independent-TIM-checker4`, **gpt-6-astra / high**, Codex
through T3 Code. This reviewer implemented none of the candidate, used no children,
and changed only review evidence and perturbation fixtures. No production, account,
comparator, baseline, skip rule, target, historical review or reference was edited.
No local Java/javac, SSH, remote agent, push, PR, release or merge was performed.

## Exact scope and provenance

The prepared checkout switched to `review/tim-adversarial-cb14b5f3` at exact
`ee0e4c72fb222f1312d1aaf7d5a152cc97f12781`, production
`6c550246b458eb5f37539713510dc7d919843421`. The three corrective accounts
`8765ccc2`, `3020a0aa`, and `56a95428` are verified ancestors of production.
The only production file changed since `7bb210e9` is `preprocess/mod.rs`.
The whole module was read. Named Java methods and actual data flow were inspected;
no new fixture-label predicates, golden reads, coordinate surgery, or unexplained
parity constants were found. Source-derived registration names are not evidence
that their functions are implemented. Date-width modules are unchanged and were
not gratuitously re-audited. No class/sequence geometry candidate was imported.

Baseline collateral measurements reuse the preserved checker3 `after-*` captures:
production at `579b9b11` is byte-identical to `7bb210e9` across crates, vendor and
Cargo inputs. All 1,516 sources/references were verified against `463fc9bd` and its
frozen inventory; **411 missing pairs were restored byte-identically**. The fresh
31 cases also ran against a separately built `7bb210e9` source archive, in a
separate Cargo target. No source-mtime artifact reuse or cache cleaning occurred.
Candidate/baseline binary hashes and all archived build-input hashes are recorded.

Fresh Java captures ran only through the parent's checksummed byte relay to the
existing remote pinned wrapper. Java source:
`71806a23780b04a5ccde2f8ceb5121edad5eb711`; JAR SHA256:
`aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
Workers were `worker.oeM25myU` (29 cases) and `worker.HCCMlVMz` (2 follow-ups).
Every immutable source/resource, input-manifest membership, returned stream hash,
compressed bundle, cwd, TZ and synthetic environment override was independently
verified. All raw SVG calls used exact stdin bytes and `-pipe -tsvg`; source probes
are supplemental. Both capture modes used each case's declared environment.

Remote OpenJDK 21.0.5 / Graphviz 12.2.0 differ from origin 21.0.13 / 16.1.0.
Pinned UTF-8/en_AU/headless and matching GMT+11:00 establish this scope, not global
runtime equivalence. No ambient environment values were requested. The only extra
environment entries were explicit benign synthetic controls.

## Causal findings

1. **Typed string boundaries still lose UTF-16.** In
   `checker4_surrogate_case_conversion`, Java and the baseline produce `0 / 0`;
   the candidate produces `-1 / -1`. Substr now retains surrogate halves, but
   `eval_builtin_values` converts them through lossy display strings before
   `%upper`/`%lower`. Java keeps UTF-16 units through case conversion. This is a
   lost CLI pass even though the old substring behavior passed accidentally.
   Separately, `%chr("66")` and JSON numeric arguments are parsed from display
   text by Rust; Java `Chr` uses `TValue.toInt`, giving zero for either type.
2. **Arity errors are missing.** `%strlen("a","b")` should reject. With a later
   invalid date, Rust reports that later date error; without it, Rust exits 0 and
   emits `class After`. The first-error latch cannot preserve a failure that was
   never created. Java function resolution/`canCover` enforces arity.
3. **Named operands are not fully snapshotted.** When `$a` is also global, Java
   expression RPN resolves the named-key operand through `Knowledge`, but plain
   `EaterFunctionCall` reads it literally. The fresh control produces Java
   `seed/4 / elm/5` versus Rust `oak/4 / elm/5`. Rust extracts the named key before
   preparing its value, erasing that distinction.
4. **Substitution has a later rescan.** A variable and return value containing a
   `!definelong` name stay literal in Java. Rust's `substitute_vars` preserves
   them, then `try_expand_definelong` scans the completed output and expands them.
   Separately, `VariableManager.justAfterBackslashN` allows a direct legacy token
   after `\\n`; Rust's preceding-letter test prevents that substitution.
5. **Local end markers can erase the model.** Java recognizes `!endsub` followed
   by a suffix via `TLineType.ENDSUB`. Rust local collection requires exact string
   equality, swallowing the remainder of the source. The valid Java class `n=4`
   becomes an empty Rust Sequence model with no participants/events. External
   selection already uses a different boundary parser.
6. **Raw sub selection removes comments too early.** `UncommentReadLine` strips
   pre-start header/pause prefixes; it is not a TIM block-comment remover.
   Java `Sub.fromFile` validates a malformed marker in the fresh raw comment
   control, while Rust suppresses it and succeeds. Selected-body execution and
   raw structural scanning must remain separate.
7. **Collection and registration contracts remain incomplete.** JSON scalar
   foreach throws in Java `EaterForeach.size`; Rust treats it as empty and emits
   the following class. `%boolval(0)` is registered and valid in Java but fails
   in Rust's missing implementation branch. This is a **new valid-input CLI
   error**, not a lost SVG pass: baseline output was already wrong.

Bounded repairs should preserve typed UTF-16 through each builtin, enforce actual
arity/contracts, retain the expression/plain named-key distinction, remove the
post-substitution macro rescan, share source-derived sub marker boundaries, and
validate raw markers and collection types at Java's actual execution layer.
Do not tune renderer coordinates, restore silent-empty fallback, or relax gates.

## Supported observations and negative scope

Fourteen fresh cases match both exact source and actual CLI SVG. They corroborate
string/JSON defaults, nested typed conversions, expression versus plain-call
argument timing, eager conditions, JSON member value types, define capture once,
nested loop-return unwinding, sibling frame restoration, JSON global iteration
binding, insertion order, source-collection snapshots, repeated external sub
selection, and filtered environment lookup under LEGACY and UNSECURE.

These are bounded observations, separate from acceptance. The seven new account
reviews all record rejection; the return-unwind record names its successful slice
without inventing a fresh unwind defect. Older reviews are preserved and are not
retrospectively re-attested. The full existing union was replayed as collateral.

There are **10 negative references**, not ten passes. Four are rejected by Rust
(one with the wrong first error); six are accepted. These include the Java-invalid
function-in-index grammar, missing nested JSON member, scalar foreach, malformed
raw marker, wrong arity, and isolated-surrogate output. The isolated-surrogate
Java command exits **0** but returns an **ERROR SVG** with a serialization exception;
it is correctly classified negative. Its source probe's `?` UTF-8 projection does
not establish successful Java SVG rendering. Known floating/unary/no-return
extensions and arbitrary JVM property configuration remain explicitly unproved.

## Measurements and gates

The **actual CLI**, unchanged comparator, and matching per-case environments give:

| Frozen cohort | Baseline matches | Candidate matches | Lost passes / new valid errors / new panics |
| --- | ---: | ---: | ---: |
| Parent 1,449 | 520 | 520 | 0 / 0 / 0 |
| Parent + prior checker 67 = 1,516 | 553 | 567 | 0 / 0 / 0 |
| New independent 31 | 1 | 14 | 1 / 1 / 0 |

The historical 955 and 1,044 subsets remain 431 and 495 matches respectively.
The maker's available 1,105 are a subset of 1,516; the 411 restored cases are the
exact set difference, never an added pass count. Maker's 17-case reference batch
is a separate retained evidence cohort, not added to these denominators: all 20
resource hashes and raw bundle streams were reverified; its 14 valid archived
SVG/source agreements were reproduced with the unchanged comparator and proven
same production source. No new Java or CLI execution is claimed for that reuse.

The 1,516 retain 57 invalid references and 32 pre-existing panicking enforcement
entries. Those entries cover **two underlying sequence counterexamples**, repeated
with suffix/environment controls, at the same underflow site (24 len-2 and 8 len-1
entries). They are not 32 independent new bugs or a registry cascade. All 48 valid
prior checker cases with retained YAML have exact current Java source agreement
and no entity-count drops. This scoped check does not excuse the fresh local-sub
model loss or unmeasured models.

Both main release tiers ran **once** on unchanged final candidate production:
product **11179/11251**, assisted **10497 passed / 754 failed / 1299 skipped**.
Both are red. Historical approved GET-versus-raw losses remain explicit:
`activity/act_skinparam2_shadowing_true`, `activity/act_skinparam_shadowing`, and
`edge-cases/edge_whitespace_multiple_blank_lines`. All six raw-reader controls pass
file mode and collateral pipe mode; all five class/TIM controls pass source/SVG.

Parser **590/590**, CLI **15/15**, candidate rustfmt and diff whitespace pass.
Workspace clippy fails on unchanged `parse/component.rs:72` dead code.
Guards are **4/6**: decimal ratchet and review evidence remain red. New review
records have no schema/path/missing-pair failures; they intentionally reject and
retain negative/failing pairs. Historical missing account metadata stays reported.

The unchanged standing perturbation gate is **555/1547**, red. Matching-environment
isolated CLI is **581/1547**. All 26 differences are listed: declared include/theme
cwd, named/fractional zones, and two synthetic getenv environments unavailable to
the fixed gate. These environment mismatches are not new product defects and do
not waive the standing gate. Neither suite's rules were altered.

The prior retained sub case still fails SVG despite exact source: class source_line0
fallback/entity ordering remains. Class palettes/quoted aliases, activity,
handwritten, sequence dependencies, and second-preprocess-pass-only dynamic-error
coverage remain unwaived. No combined renderer candidate was tested.

## Durable evidence and reproduction

Seven sibling `review.json` files own every new pair; all 31 sources, same-stem
raw Java SVGs (including empty/error outputs), and four resources live under their
mechanism in `test-diagrams/perturbations/`. All prior inputs/reviews are untouched.
`fresh-summary.json`, per-case result files, membership, provenance, and validation
records distinguish statuses and denominators. `measurement-artifacts.tar.gz`
retains all measured CLI SVG/YAML/stderr/comparator output, Java streams, controls,
and full gate logs. The two compressed remote bundles are included verbatim.

The persistent private cache is
`/Users/marcelo/Library/Caches/rustuml-review-cb14b5f3`. Captures were staged there,
then deliberately packaged. Scripts and exact commands are retained here. Restore
archived captures there to inspect without rerunning Java. `evaluate.py` and
`evaluate_supplement.py` verify relay bytes and run the local CLI/comparator;
`replay.py collateral` uses original frozen memberships. `provenance.json.gz` pins
separate baseline build inputs and current binaries. Build targets are private;
no whole-cache clean is part of reproduction. Read `commands.json` beside each
capture for its exact remote invocation, cwd, environment and status.
