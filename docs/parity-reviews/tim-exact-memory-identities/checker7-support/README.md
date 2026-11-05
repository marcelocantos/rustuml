# Independent TIM review, candidate 2cf6c7ee

All three mechanism reviews are **rejected**. Exact-key storage and the tested signature/collector changes repair real causes, but fresh source-guided inputs expose three valid TIM failures, two valid downstream class-parser failures, and two empty-sub termination differences. Bounded supported behavior does not waive standing gates or unrelated failures.

The reviewer was a separate T3 task using **gpt-6-astra, high reasoning, Codex**, with no children and no implementation edits. Full task identity is in each `review.json`. This evidence-only branch was created from the clean T3-prepared checkout at `2cf6c7eeebae7354e7a3b7e53b86fb3754883e67`. The full preprocessor, review schema, three precommitted accounts, maker report, Java data flow, and Rust/Python companions were read. `oracle-first` guided source hypotheses, observable product behavior, and separate validity classification.

## Identity and chronology

| Role | Exact revision |
| --- | --- |
| Before | `4a288dd47fe925a792f7cb672ca294f6ef6fdcf3` |
| Prior checker6 evidence | `a3b48251354497c68eaa0b93a168d3ba5c455c0f` |
| Imported prior evidence | `ba57d3c7275a688b1f1ac565551cdff08fc52d32` |
| Preimplementation accounts | `4652860936464f02d80584ef158a29a4a1c13cd1` |
| Main production correction | `329c39f48eccddfbb4896f1710dd97ddd1c1f2e8` |
| Filename restoration / final production | `55de7dd6cb090d86febc1e5e3f57804134429a19` |
| Reviewed candidate | `2cf6c7eeebae7354e7a3b7e53b86fb3754883e67` |
| Java source | `71806a23780b04a5ccde2f8ceb5121edad5eb711` |
| Golden gitlink | `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a` |

`revisions.json` records commit chronology and binary/comparator hashes. Candidate production is byte-identical to final production; the before-to-candidate production diff changes only `crates/rustuml-parser/src/preprocess/mod.rs`. All 9,170 candidate historical evidence files were checked against their original Git blobs before and after evidence packaging, with zero changes. New reviews and fixtures supplement history; nothing is rewritten.

Golden hydration initially failed because Git disallowed the local file transport. A per-command `git -c protocol.file.allow=always submodule update --init --checkout` hydrated the exact gitlink (25,106 files). No configuration or corpus changes were made. An empty checkout was not treated as a passing test corpus.

## End-to-end causal audit

**Memory.** Java `EaterAffectation`/`EaterAffectationDefine` preserve the parsed name when inserting a `TValue`; `TMemoryGlobal` updates its exact-key map and trie. `TMemoryLocal` resolves overrides, globals, then locals; get, remove and existence must use the same exact identity. `TFunctionImpl.getNewMemory` forks from global memory, so a nested invocation is a sibling frame rather than a lookup through the caller's locals. `Eater.eatDeclareFunction` evaluates defaults at declaration and retains each formal's complete name. Expressions reach `Knowledge` through `ShuntingYard`; text reaches `VariableManager` through `TContext.applyFunctionsAndVariables`, scanning original text without rescanning appended values. The correction removes the old dollar stripping and fixes the sampled ordinary cases.

That is not the whole model. In `key_dollar` and `key_combining`, `SetVariableValue` creates a name admitted by Java's exact string map/trie (`a$b`, or `twig` followed by U+0301). Direct get and existence work in Rust, but `substitute_vars` at lines 2590–2605 only scans alphanumeric/underscore tokens. It cannot match the full stored key. Java evaluates the label to `spruce / spruce / 1`; Rust retains the key in the middle. These are valid CLASS-factory diagrams, not negative syntax probes.

In `parameter_duplicate`, Java `TFunctionImpl.getNewMemory` lines 99–121 performs `containsKey/get` for **each** declared formal and overwrites the final result map entry. Rust `call_function_values` lines 1581–1593 consumes the named entry with `remove`; the second same-name formal therefore takes its default. Java emits `named / right`; Rust emits `two / right`. The positional control still matches. All three failures already occur before the correction; they disprove completeness, not a claim that this patch introduced them.

**Signatures.** `FunctionsSet` retains a pending definition separately, publishes on endfunction, and replaces completed entries by `TFunctionSignature` (name and declared arity; named arguments do not participate in equality/hash). `getFunctionSmart` performs exact lookup before traversing `canCover` fallbacks. `EaterFunctionCall` supplies named keys for plain calls; RPN uses positional count and empty named coverage, then checks execution kind, then binds parameters. The completed-name trie controls textual call recognition. Rust `register_function`, `resolve_function`, `call_function_values` and the text scanner now follow these bounded paths. Six fresh valid cases have exact evaluated-source equality. The procedure-in-expression negative rejects after exact selection, rather than incorrectly falling back to a return function.

`signature-audit.json` independently reconstructs all **74** active standard registrations from pinned `TContext.addStandardFunctions`, excluding commented `LoadJsonLegacy`, and checks the name, arity and original bytes of every builtin class. The inventory matches the maker derivation; it is not accepted merely because its length is 74. Hash initial 1 and multiplier 31 come from `TFunctionSignature`; the xor-shift of 16, capacity 16, 3/4 load and doubling come from the recorded OpenJDK `HashMap` source. The local JDK source SHA is `71c8d82247c2736e9fbe3769f366e9bd250bc73ecf390749998e830201aaabf2`.

Fresh `capacity_20`/`capacity_21` vary one independent registration across total occupancy 96/97. At capacity 128, `$oak515` arities 2/3 occupy buckets 127/0 and Java picks `three`; at capacity 256, they occupy 127/128 and Java picks `two`. Both source and SVG match after correction. Maximum final bucket occupancies are 4/3. This checks ordinary bins, not tree bins. `min_by_key` plus same-name insertion order models ordinary bin traversal only. HashMap does not promise iteration order: tree root promotion, identity-dependent ties, JVM implementation differences and registration history can matter. Rust's separate legacy registry can also misrepresent same-name/arity collisions and legacy overload occupancy; unsupported legacy syntax and `%function_exists` coverage remain unproved boundaries. No global registry equivalence is approved.

`select_0`/`select_1` vary declaration order, exact-vs-defaulted arity, named fallback, and bare-name boundaries. They have **exact Java/Rust evaluated source** but both valid SVGs fail downstream. Java `CommandCreateClass` uses `NameAndCodeParser`'s complete display/code command. Rust `parse/class.rs::try_entity_decl` at lines 679–693 accepts a prefix with `[^\"]+`, stops at the inner quote in `xpick("skip")`, and loses the rest of the label and `as Node` alias. The YAML entity is truncated; this is not a TIM lookup mismatch. The original fixtures remain unchanged and failing. A downstream cause does not waive their product result.

**Sub collection.** `TContext.buildCodeIterator` composes comments, then `CodeIteratorSub`, then return-function, procedure, conditionals and legacy-definition iterators. `CodeIteratorSub.peek` drains and checks a sub body, stores its located lines, and installs a replay iterator before any later collector consumes them. Nested declarations fail during collection, before body effects. Rust now collects before its function/procedure/definelong paths and replays `BufferedLine` origins through `process_buffered_line`. Six fresh nonempty cases exercise collectors, delayed effects, inactive branches and comment/location handling; all match source and SVG. The nested negative is rejected.

The empty-body case differs: Java installs an empty `CodeIteratorImpl`; `CodeIteratorSub.peek` lines 68–97 returns null, and `TContext.executeLines` terminates its peek loop. Later lines and `@enduml` disappear. Rust's empty vector loop at lines 1006–1012 completes normally and later source continues. Both `sub_empty_0` and `sub_empty_1` therefore produce Java source exactly `@startuml\nclass Anchor\n` and a welcome SVG, while Rust emits the tail class/edge. These are preserved as observed empty-sub truncation outcomes, **not** valid CLASS positives, even though both Java processes exit 0.

**Downstream source oracle.** TIM output travels through `TContext.resultList` → `TimLoader.load` (including its error flag/catch behavior) → `BlockUml` data/Jaws and diagram factory selection → `getDiagram().getSource().getPlainString()` in unchanged `SourceProbe`. The SVG CLI also goes through factory/error handling. A successful source probe can therefore describe a preprocessor error or truncated block; exit 0 alone establishes no validity. Per-file pinned hashes are in `java-provenance.json`.

The production diff contains no fixture-label dispatch, golden reads, or post-hoc SVG surgery. New numeric/hash behavior is source-derived. This source audit does not turn failing mechanical guards green.

## Ordinary assertions and filename recovery

`ordinary-units-audit.json` verifies the original input expressions and old/new assertion blocks against exact revisions. The archived `logs/historical-parser1.log` preserves the maker's five original failures (600 passed, five failed); the independent `original-units/` captures run each **unchanged original input** through both CLIs, including the exact included `defs.puml` content.

| Unit | Old assertion | Source-justified corrected assertion |
| --- | --- | --- |
| define_substitution | `Alice -> Bob : hello` | `$Alice -> Bob : hello` |
| multiple_vars | `Alice -> Bob : hello` | `$Alice -> $Bob : $hello` |
| include_with_defines | `Alice -> Bob : hi` | `$Alice -> Bob : hi` |
| foreach_preserves_other_vars | `Alice` as sender for hello/world | `$Alice` as sender for both |
| if_else | `A -> B : production` | `A -> B : dev` |

`!define NAME` stores bare `NAME`. During text scanning, the unmatched dollar remains literal and the subsequent bare name expands; include processing shares that memory and foreach binds only its own exact key. In an expression, `$MODE` is a distinct absent key and remains its name, so it is not equal to `prod`. These are independent source derivations per original unit. The retained maker supplement corroborates the evaluated source, with 2/2 exact-source matches but only 1/2 SVG matches because of disconnected packing. No blanket approval follows from the 615 passing parser units.

`filename-source-audit.json` proves the filename branch at final `55de7dd6` is verbatim the before branch, including old comments, and absent at intermediate `329c39f4`. `filename-cli.json` and raw streams in the archive independently verify the original frozen `preproc_builtin_filename` CLI pair passes before and after. The intermediate failure remains historical evidence. This establishes recovery of that real pipe regression, not every possible file API semantic.

## Frozen CLI replay

Both release CLIs were built independently with `CARGO_BUILD_JOBS=4` in separate private targets. Before source was extracted by `git archive`; no shell-created worktree was used. Build logs and exact binary hashes are retained. Runs use the shipped `-pipe` SVG and `--yaml -pipe` model paths, stdlib `/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib`, debug `date=1774210426000,tz=AEDT+1100`, and recorded cohort-specific cwd/environment overrides. Exact source is compared only where an oracle exists. Every empty stream, failure, panic, model, comparator result and real exit is retained.

The unchanged comparator binary was copied from the maker cache and hash-verified (`43ce27167f6ad8697a8043edbb4f580ddbf9ab68b4e0ea4db15f2a69e3fc97b5`), not independently rebuilt. It uses the existing strict comparison and error-golden classifier; no comparator or policy changes were made. Membership and source/reference byte hashes come from the frozen prior evidence, not this reviewer's success inventory.

| Frozen cohort | Entries | Valid | Before SVG | After SVG | After exact source / available | After negative rejection |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| collateral1516 | 1516 | 1459 | 567 | 567 | 59/59 | 37/57 |
| prior31 | 31 | 21 | 21 | 21 | 21/21 | 8/10 |
| prior fresh29 | 29 | 14 | 14 | 14 | 14/14 | 12/15 |
| retained maker15 | 15 | 9 | 9 | 9 | 9/9 | 6/6 |
| retained maker17 | 17 | 14 | 14 | 14 | 14/14 | 3/3 |
| retained maker18 | 18 | 8 | 8 | 8 | 8/8 | 10/10 |
| checker6 fresh29 | 29 | 19 | 12 | 17 | 17/19 | 10/10 |
| current maker15 | 15 | 14 | 2 | 14 | 14/14 | 1/1 |
| current supplement2 | 2 | 2 | 0 | 1 | 2/2 | — |

The 1,672 entries contain 1,593 unique source/reference pairs. All 79 duplicates are within collateral1516; intersections between the named cohorts are empty. No valid SVG pass is lost and no new valid CLI error appears. Historical retained subsets reproduce 431/955, 495/1044 and 520/1449. Additional raw-file six and retained class controls pass before and after.

Real residue remains: checker6's two Unicode `IntVal` CLI errors; the class-body ownership parse error; and 32 frozen sequence panic entries at `sequence.rs:8022:37`, grouped into two signatures (24 with len 2, eight with len 1, both index `18446744073709551615`). These failures remain executable and unaltered. The supplement's second exact-source match still has a downstream disconnected packing mismatch.

## Fresh23 provenance and classification

The fresh matrix was generated **after** implementation from source branches, varying independent axes listed in immutable `new-manifest.json`. Request schema is maker TIM7; `SourceProbe.java` is unchanged (SHA `fa9dde4b9f18e337741c73fdc23fa8d94034d2341725566e378eab0ac95ad4b4`). All 23 sources plus that probe are listed with bytes, SHA and relative paths. No local Java/javac, SSH, install, new agent, or actual-secret controls were used. Only the parent's existing Monkey Man runner produced references.

Request SHA: `711593b030a9b55c74fc0bd8ff4bc22de44ea5d32f8da919b6fde76a8f11dd57`.
Manifest SHA: `c5fa8d3e532b53c7c91cef83b43c39a4ba77645290e733e70f576d5cc9f9ec1b`.
Input gzip SHA: `b5e004cf75dece06cc5d9d2a60acc22c836dda6d53027202e8aaed7751a03ad8`.
Returned gzip SHA: `221e9e974b5b2b622fa732e7f2f7449722bb535627d19cbd814553cad4c85d30`.
Pinned headless JAR SHA: `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.

The independent validator checked current bytes of all 24 resources, 23 memberships, all 46 wrapper/Java calls, cwd `<worker>/new`, GMT+11:00, empty per-case environment overrides, jar identity and every decoded output stream hash. Actual worker was `/Users/marcelo/rustuml-reference/scratch/worker.8JVbLwLj`; wrapper SHA `6115a3503877999796cb98b9efb6872d9bb8090c2f7b986706919e475dee069e`. An initial local validator assumed reversed probe classpath order and stopped before product replay; its assertion log remains archived. The validator was corrected to the returned exact jar-first order; references were not regenerated. The original progress log also used the overly broad label `error_svg` for every comparator rejection, including welcome SVGs. Final classification metadata uses `comparator_rejects_reference` and a distinct `welcome_after_empty_sub_truncation` category; the original log and all streams are preserved, and no product or oracle call was repeated for that label correction.

Remote Java **21.0.5** and dot **12.2.0** differ from historical Java 21.0.13/dot 16.1. No global cross-environment equivalence is asserted. There were 21 SVG exit-0 calls, two SVG exit-200 calls, 23 probe exit-0 calls, and no timeouts. Classification used XML, factory, error classification and complete evaluated-source inspection, not these process counts.

| Mechanism | Observed valid CLASS | After exact source | After SVG | Other observed outcomes |
| --- | ---: | ---: | ---: | --- |
| Exact memory | 7 | 4/7 | 4/7 | None |
| Signatures | 6 | 6/6 | 4/6 | One syntax negative, rejected |
| Sub buffering | 6 | 6/6 | 6/6 | One syntax negative, rejected; two welcome/truncated-source results, both diverge |
| Total | 19 | 16/19 | 14/19 | Four nonpositive references preserved |

All 23 original sources and unchanged same-stem Java SVGs are under their mechanism's `checker7/` directory. The two negatives have semantic rejection success but are **not** positive SVG matches; their review `result` remains `fail` under the existing positive-reference suite. Welcome references likewise remain failures. No fixture is repaired, narrowed, dropped, or converted to a fake positive. Raw Java source/SVG stdout/stderr and commands are in the archive; fresh JSON retains intended and observed validity separately.

## Required gate capture and obligations

All required gates were run **once at exact candidate**, after hydration and before new evidence was added. `gates2.json` records full argv, cwd, environment, duration and real exits. No broad gate was repeated for capture arrival or evidence-only changes.

| Gate | Result | Exit |
| --- | --- | ---: |
| Parser release library units | 615 passed | 0 |
| CLI pipe integration | 15 passed | 0 |
| Main product library diagnostic | 11,179/11,251 eligible | 101 |
| Main assisted library diagnostic | 10,497/11,251 eligible; 1,299 skipped of 12,550 | 101 |
| Standing perturbations | 592/1,605 | 101 |
| Harness guards | 4/6; decimal ratchet and review evidence fail | 101 |
| Parser clippy, warnings denied | Existing unused `looks_like_description_bracket_command` at component.rs:72 | 101 |
| Touched preprocessor rustfmt | Passed | 0 |
| Candidate diff whitespace | Passed | 0 |

The two main tiers above are **library diagnostics**, not CLI headline parity. Matching-environment CLI replay yields 619/1,605 for standing membership, with exactly **27** discrepancies against the standing run. `analysis.json` enumerates every source, environment, cwd and standing failure. They remain real unresolved environmental discrepancies, not exclusions or acceptance credits.

Repair obligations are bounded and causal: implement trie-equivalent text lookup for admitted exact names; preserve named values across duplicate formal binding; model empty replay termination through factory selection; repair the downstream full-command class parse exposed by two fresh valid cases; establish tree-bin/legacy boundaries before wider claims. Preserve all prior failures while resolving standing gates, class-body ownership, Unicode IntVal, sequence panic and environment residue. No target, policy, comparator, threshold, skip or baseline change is authorized by this review.

## Durable evidence and replay

`measurements.tar.gz` holds the complete private evidence tree excluding build targets/binaries, the before source archive and unused full JDK source. It includes all frozen/fresh input and oracle copies, both CLI stdout/stderr sets, YAML/models, comparator captures, membership/result JSON, original-unit/filename controls, build/gate logs, exact remote request/transport/results, and execution scripts. `capture-manifest.json` hashes every archived file by relative path and byte count; `archive-integrity.json` records the archive hash and successful byte-for-byte verification of every extracted member.

Scripts preserve original absolute paths as provenance. Their private cache root was `/Users/marcelo/Library/Caches/rustuml-tim7-checker`; repository resources are pinned by the candidate, and external prior cache paths remain named in frozen membership. Rebuild the two exact revisions into separate private targets before rerunning the recorded CLI argv. Fresh standalone `.puml`/`.svg` pairs are directly executable through the existing perturbation harness. Do not run remote reference generation locally or reinterpret absent source oracles as source matches. Evidence packaging checks only hashes/structure/whitespace and does not manufacture a new product gate result.
