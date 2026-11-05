# Completion of the 17-case TIM maker reference batch

Candidate `6c550246b458eb5f37539713510dc7d919843421` matches all **14 valid
fresh Java SVGs and all 14 exact Java UmlSource streams**. The three negative
controls reject in both implementations. No production change was needed.
This completes the pending reference capture recorded in evidence commit
`7cae38b255a62f41f1e0234b646899426fb67316`; that historical snapshot is unchanged.
This is maker evidence, not an independent review or acceptance verdict.

The batch covers typed builtin contracts and UTF-16 surrogate round trips,
variable snapshots across nested calls and JSON indexing, literal/define capture,
nested loop return unwinding, JSON foreach values/global bindings/object order,
expanded sub selectors/local blocks/marker validation, and builtin registration
and filtered environment lookup. `results.json` maps every case to its account.

## Transport and verification

SSH remained unavailable. The parent relayed exact bytes through the working T3
channel to the existing Monkey Man runner. The remote runner verified all 20
resources before its 34 calls (17 raw SVG calls and 17 source probes), with no
timeouts. Evaluation here independently verified:

- The output compressed SHA-256, decompressed JSON bytes, 17-case membership and
  every input SHA-256.
- All 20 resource sizes/hashes against the immutable request and current original
  files; diagram/include bytes also match the previously committed inputs.
- Strict base64 decoding of every returned stdout/stderr stream.
- The candidate binary and unchanged comparator hashes against the prior evidence.

Transport input compressed SHA-256 (reported by parent and remote environment):
`4cd171c5c3b0e08205875b4e845f7434cd03c41a6ac875ea539541218f400539`.
Verified output compressed SHA-256:
`cd50f6bf14cd85993a8e584280d4db3c45133ced6a74ca6682c0d3da0704f7ae`.
The exact compressed result bundle is preserved as `tim17-results.json.gz`.

Actual remote SVG command, with each unchanged input's bytes on stdin:

```
/Users/marcelo/rustuml-reference/bin/reference --cwd /Users/marcelo/rustuml-reference/scratch/worker.5NhExUKP/new --timezone GMT+11:00 -- -pipe -tsvg
```

For example, the corresponding first source probe was:

```
/Users/marcelo/rustuml-reference/bin/reference --cwd /Users/marcelo/rustuml-reference/scratch/worker.5NhExUKP/new --timezone GMT+11:00 --probe /Users/marcelo/rustuml-reference/scratch/worker.5NhExUKP/SourceProbe.java --classpath /Users/marcelo/rustuml-reference/scratch/worker.5NhExUKP -- /Users/marcelo/rustuml-reference/scratch/worker.5NhExUKP/new/builtin_window.puml /Users/marcelo/rustuml-reference/scratch/worker.5NhExUKP/new
```

`remote-environment.json` preserves the exact worker, commands, hashes and runtime:
OpenJDK 21.0.5, Graphviz 12.2.0, en_AU, UTF-8, GMT+11:00, headless AWT; pinned JAR
SHA-256 `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
The runner removed ambient JVM option/classpath variables; the unchanged pinned
wrapper supplies headless configuration. These runtime versions differ from the
origin environment, and scoped agreement does not establish global equivalence.
No local Java or javac was launched.

## Classification and retained failures

| Case | Java SVG exit | Java probe exit | Rust CLI exit | Observation |
| --- | ---: | ---: | ---: | --- |
| 14 valid cases | 0 | 0 | 0 | Strict SVG and exact source agreement |
| builtin_negative_length | 1 | 1 | 1 | Java throws in Substr; first substring error wins in Rust |
| procedure_value_error | 200 | 0 | 1 | Procedure-as-value rejects; Java error SVG retained |
| sub_bad_unselected | 200 | 0 | 1 | Unselected malformed marker rejects; Java error SVG retained |

Negative controls are not SVG successes. Java's empty substring output, exception
stack, syntax-error SVGs and error-source streams are all preserved. Rust diagnostics
are also retained. The sub-marker diagnostic reports Java `ERROR / 1` versus Rust
`line 0`; error formatting and numbering are not claimed identical. There are no
valid-input CLI errors, valid SVG mismatches or valid source mismatches in this batch.

Each case directory contains exact Java SVG/source/stdout/stderr, actual Rust
SVG/YAML/source/stderr, comparator output and command/status records. The source
probe reads the full selected source through SourceStringReader; it is supplemental
to the shipped CLI SVG comparison, not a replacement for it. No input normalization,
reference regeneration, comparator change or baseline change was used.

## Reproduction and handoff

`evaluate.py` performs only local hash checks, CLI rendering and strict comparison
against the captured relay outputs. Run it from the checkout root with the original
immutable `/tmp/rustuml-tim4/new-manifest.json`, resources and cached bundle present.
It requires the preserved candidate binary at `/tmp/rustuml-tim4/final-rustuml` and
unchanged comparator at `/tmp/rustuml-tim4/probe`; both hashes are checked before use.
It writes scratch results under `/tmp/rustuml-tim4/relay-completion/`. No remote
reference execution is needed to reproduce the comparison.

No full corpus, main tier, parser suite or CLI suite was repeated because production
is unchanged. Their results remain those in the previous maker report: 14 checker3
gains with no lost passes/new valid errors in the available 1,105-case replay;
parser 590/590, CLI 15/15; main product 11179/11251 and assisted 10497/11251; guards
4/6. The two sequence panics, unowned class source_line0 fallback/entity ordering,
style/palette/activity/handwritten dependencies and combined integration work remain.
The fresh surrogate round-trip control now has actual Java evidence; arbitrary
isolated-surrogate final display and JVM system-property configuration remain
outside what this batch establishes.

All historical reviews, sources, references and accounts remain unchanged. A new
independent checker is still required; no acceptance record has been authored.
