# TIM checker4 corrective maker packet — acceptance pending

Production is `92647b126f6dc1c83ebdc6213d0c2c0a5b3211b6`, following six causal
accounts committed **before implementation** at
`6d46a483aad389d4535e1ba24822fdd116baf517`. This is maker evidence, not an
independent review or acceptance record. Parent has not integrated this production.
No review, previous account, original fixture, comparator, baseline, exclusion,
reader policy, date-width module, parser handoff, renderer, or target was changed.

The exact candidate parent was `ee0e4c72fb222f1312d1aaf7d5a152cc97f12781`.
Checker evidence `0d30ae99e85195f595e21ca0a80be85f26473ee9` was imported as
`47a723b6fd424a2584dc3d89ac0dc0328eeb0ae7`, including all 411 byte-identically
restored prior pairs and 31 new pairs. Packaging verified all **6,967 existing
tracked evidence/fixture files** against that import's Git blobs. All 1,547 source
and reference pairs are separately hashed in `frozen-membership.json.gz`.

The six new accounts are `tim-builtin-value-boundaries`,
`tim-call-resolution-arity`, `tim-named-key-operands`,
`tim-original-macro-scanning`, `tim-sub-marker-layers`, and
`tim-json-container-validation`. Each contains exact named Java data flow and two
or more perturbation axes. Production changes only `preprocess/mod.rs`:

- Preserve UTF-16 code units through upper/lower; use `TValue.toInt` for Chr and
  BoolVal's explicit textual true/false/1/0 conversion.
- Validate implemented function arities after argument evaluation and before
  executing their bodies, preserving the first actual failure.
- Snapshot RPN named-key operands through memory lookup; plain-call keys remain
  literal, as in `EaterFunctionCall`.
- Recognize legacy calls while scanning original source, avoiding a completed-output
  rescan; honor the literal backslash-n boundary. Bare definelong names do not call
  functions. Existing legacy body lowering remains explicitly limited.
- Use source-derived local endsub boundaries and scan external raw structural
  markers before TIM comment execution.
- Reject JSON scalar foreach and continued traversal through missing JSON members;
  preserve non-JSON foreach skipping and a terminal missing member's empty result.

The old indentation unit assertion was corrected from two to four spaces:
`TContext.applyFunctionsAndVariables` retains the two-space caller prefix, and
`TFunctionImpl.executeReturnLegacyDefine` returns the two-space body prefix
verbatim. The old post-substitution helper trimmed the caller. The failed interim
unit run is retained; no frozen reference was edited.

## Actual CLI observations

| Cohort | Prior candidate matches | Current matches | Lost passes / new valid CLI errors |
| --- | ---: | ---: | --- |
| Frozen parent 1,449 | 520 | 520 | 0 / 0 |
| Restored 1,516 | 567 | 567 | 0 / 0 |
| Checker4 fresh 31 | 14 | 21 | 0 / 0 |
| New maker 15 | 3 | 9 | 0 / 0 |

The 31 checker cases contain **21 valid references and 10 negative references**.
All 21 valid cases now match exact Java source and unchanged strict CLI SVG.
Eight negatives are rejected. Java-invalid function-in-index grammar and isolated
surrogate output remain outside this bounded claim; Rust still accepts them.
Java's isolated-surrogate case exits zero with an ERROR SVG, so it remains negative.

The new 15 contain **nine valid references and six negatives**. All nine valid
cases match exact source and CLI SVG. Rust rejects all six negatives. Five Java
errors exit 200 with well-formed error SVGs; JSON boolean foreach exits 1 with empty,
non-XML output. Source probes can exit zero on error diagrams. `relay15/results.json`
records XML validity, process status, source status and comparison independently;
none of these six errors counts as a positive SVG match.

The prior separate maker17 cohort remains **14 valid source/SVG matches and three
negative rejections**. It is not added to the 1,516 or 1,547 denominators. Historical
955 and 1,044 subsets remain 431 and 495 matches. The 48 valid prior diagnostic cases
retain exact Java source, with no entity-count drops across those and the 21 fresh
valid checker cases. Six raw-reader file controls and all five class/TIM source/SVG
controls still pass. This does not assert combined class/sequence candidate parity.

The 1,516 retain 57 invalid references and 32 panicking entries representing two
known sequence counterexamples repeated in retained controls, not new independent
bugs or registry cascades. One further unchanged valid CLI error is
`class-command-grammar-closure/retained-maker/prior31/body_ownership.puml`:
`invalid allowmixing command`. These are all retained, including cases whose old
SVG already mismatched.

## Gates and environment distinction

- Parser: **597/597**. CLI pipe suite: **15/15**. Touched rustfmt and diff checks pass.
- Main product: **11,179/11,251**, red. Assisted: **10,497 passed / 754 failed /
  1,299 skipped**, red. Each main tier ran once after the correction stabilized.
- Standing perturbations: **562/1,547**, red. Matching-environment isolated CLI:
  **588/1,547**. The same 26 cwd, named/fractional timezone and synthetic getenv
  differences remain listed in `standing-versus-isolated.json`; they are not waived.
- Guards: **4/6**. Decimal ratchet and review evidence remain red. No self-review
  was written. Clippy stops at unchanged `parse/component.rs:72` dead code.

The first standing-gate command accidentally omitted the pinned clock setting and
returned 548/1,547 with 40 environment differences. That complete failed run remains
in `logs/perturbations-unfrozen-clock.log` inside the archive. Only that gate was
repeated with `RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100`; no code, test policy or
reference changed. Main tiers were not repeated. Their three historical
GET-versus-owner-approved-raw differences remain `act_skinparam2_shadowing_true`,
`act_skinparam_shadowing`, and `edge_whitespace_multiple_blank_lines`.

## Remote reference provenance

No local Java/javac, SSH retry, networking change, child agent or shipping action
occurred. New Java captures used the parent's existing checksummed T3 relay to
Monkey Man worker `/Users/marcelo/rustuml-reference/scratch/worker.4Whf5xcR`.
All 18 original resources and 30 remote calls were verified against the immutable
request, manifest, byte hashes, actual argv, cwd, timezone and environment.

- Request SHA256: `7eb6f7446148b437cf104be02461a2fd724cf6ed4a554b7205161d3fd89dc462`.
- Manifest SHA256: `46deda6a3749d4f7af193438c7920db3ea9aa35e384f39092d2539db41caaf91`.
- Input transport SHA256: `3c8aadaec4e5354798790215cfda12668e10e2fab271d7213c788286ec8c36e5`.
- Returned gzip SHA256: `3edc36d366ff72e7b2838963926af9d2d7d11b638bb98263e1cd2aec9f7f1c33`.
- Java source: `71806a23780b04a5ccde2f8ceb5121edad5eb711`.
- JAR SHA256: `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.

Every new capture declares GMT+11:00, UTF-8, en_AU and headless AWT. Remote OpenJDK
21.0.5 and Graphviz 12.2.0 differ from origin 21.0.13 and 16.1.0. Scoped transport
conformance is not global equivalence. The exact launcher for `case_units` was:

```
/Users/marcelo/rustuml-reference/bin/reference --cwd /Users/marcelo/rustuml-reference/scratch/worker.4Whf5xcR/new --timezone GMT+11:00 -- -pipe -tsvg
```

The exact source bytes were stdin; the full wrapper and underlying Java argv for
every SVG/probe are in `relay15/*/commands.json`. Actual Java argv explicitly
includes `-Djava.awt.headless=true`. Raw returned streams, exit statuses and the
complete returned compressed bundle are preserved. Both local CLI captures and
unchanged comparator output, including trailing blank lines, live in the measurement
archive without text normalization. Same-stem Java SVGs beside
`inputs/*.puml` are exact bytes, including the empty negative reference.

## Reproduction and remaining dependencies

Build current CLI with `CARGO_TARGET_DIR=/tmp/rustuml-tim5-target
CARGO_BUILD_JOBS=4 cargo build -p rustuml --release`. The previous candidate binary
was preserved separately from `/tmp/rustuml-tim-repair-target`; no source-mtime
cache switching or cache clearing occurred. `provenance.json` hashes both binaries,
the unchanged strict comparator, production module and archived evidence.

`measure.py iteration1 fresh31` and `measure.py iteration1 collateral` run actual
CLI SVG/YAML with preserved memberships and case environments. `new_rust.py` records
both binaries for the new inputs; `relay_eval.py` only verifies remote streams and
compares those saved outputs. `prior17.py`, `analyze.py`, `summarize.py` and
`validate.py` preserve the other commands. Scripts use the recorded scratch paths;
restore `inputs` under `/tmp/rustuml-tim5/new` and the archived captures as needed.
Existing checker source diagnostics are read from its preserved private cache or
`checker4-support/measurement-artifacts.tar.gz`, never regenerated. Do not rerun the
immutable generator into an existing input directory.

Registration is distinct from execution support: `registration-audit.json` lists
**41 additional registered names without implementations**; they remain explicit
errors. No unrelated builtin breadth was added. General isolated-surrogate display,
function-in-index grammar and arbitrary JVM property configuration remain unproved,
as do broader Unicode-version/locale and legacy macro-body execution semantics.
Class source-line-zero fallback/entity ordering, palettes/style/quoted aliases,
handwritten/activity/sequence geometry and the two sequence panics remain explicit
outside-owned dependencies. No comparator slack, baseline, error rule or excluded
case was changed. A new independent checker must challenge this packet.
