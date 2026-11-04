# Independent TIM checker5 — exact candidate not accepted

Reviewed `1703a2727fa364ee8004ba59a22b4c6934ee5d3d` (production
`92647b126f6dc1c83ebdc6213d0c2c0a5b3211b6`) against independently rebuilt
`ee0e4c72fb222f1312d1aaf7d5a152cc97f12781`. Reviewer: gpt-6-astra/high,
Codex through T3 Code, isolated checkout `t3-04b6729a`. No children or production
edits. The six accounts at `6d46a483aad389d4535e1ba24822fdd116baf517` precede
production; the production commit changes only `preprocess/mod.rs`. The evidence candidate
has identical shipping sources. Twenty cited Java files match pinned revision
`71806a23780b04a5ccde2f8ceb5121edad5eb711` byte for byte.

Four mechanisms are falsified by fresh in-scope cases:

- **Named keys:** `keys_exact_spelling` is a new lost actual-CLI pass. Java and
  before show `first/last/first/blue`; candidate shows `first/violet/first/blue`.
  A prepared key value `b` must not alias parameter `$b`. Rust's unconditional
  dollar stripping destroys that distinction. Duplicate keys and nested calls
  additionally diverge from Java's expression-wide RPN named-map/stack lifetime.
- **Arity:** plain `%strlen($ignored="cedar")` has zero positional arguments in
  Java and is rejected. Rust treats the entire named expression as one string
  argument and renders `16`. A zero-argument expression call to a one-argument
  procedure also reports the wrong first error: Java resolves coverage before
  checking return-call kind; Rust reverses those operations.
- **Original macro scan:** before a multiline legacy call, Java stores the raw
  caller prefix in `pendingAdd`. `macro_original_prefix` retains `$p first` in
  Java, but candidate emits `prefix first`. The scanner needs distinct raw and
  evaluated prefix values. Java intentionally discards the multiline suffix;
  that initial suspicion was disproved and its passing control is preserved.
- **Local sub markers:** Java `CodeIteratorSub.peek` rejects nested STARTSUB while
  collecting. Rust buffers and reinterprets it during replay, accepts the input,
  and swallows the following model. Local structural validation must precede
  replay. External raw-comment validation and suffix boundaries do pass their
  bounded controls.

Builtin UTF-16 split/rejoin, typed Chr, explicit BoolVal, JSON scalar foreach
rejection, terminal missing members, and missing-chain rejection survive their
fresh bounded tests. Their records distinguish supported behavior from acceptance:
all six review verdicts remain **rejected**, because the mandatory unchanged
standing gate is red. This does not assert a new conversion/container defect.
No whole-TIM or combined-candidate acceptance follows.

Known isolated-surrogate display and function-in-index grammar failures remain
explicit, outside the maker's bounded claims. The former exits zero in Java but
returns a crash SVG; its successful source probe encodes `?`, while Rust emits
U+FFFD. The latter is Java-invalid but Rust accepts it. They limit any assertion
of complete text projection or JSON-access grammar and remain executable failures.

The adjacent implemented-dispatch audit also finds pre-existing body contracts:
IntVal rejects `12.5` in Java while Rust truncates it; HslColor uses typed integer
conversion and its fourth alpha operand in Java, but Rust parses strings and
ignores alpha. All 33 new arity-table entries match their named Java `canCover`
methods. No work was added for the 41 registered but unimplemented names.

## Independent measurements

Both release binaries were freshly compiled in separate private Cargo targets
outside Git. The before build completed before this checkout switched back to the
candidate. Neither binary was reused across a source-mtime switch. Binary hashes,
module hashes, exact commands, cwd, explicit environment, statuses and streams are
preserved in the manifests and measurement archive.

| Frozen membership | Before CLI matches | Candidate CLI matches |
| --- | ---: | ---: |
| Parent 1,449 | 520 | 520 |
| Restored 1,516 | 567 | 567 |
| Previous checker 31 (21 valid) | 14 | 21 |
| Maker 15 (9 valid) | 3 | 9 |
| Previous maker 17 (14 valid) | 14 | 14 |

All valid previous-checker and both maker cohorts also match exact evaluated
source. Previous-checker negatives reject 8/10; maker15 rejects 6/6 and maker17
3/3. The full 1,516 have no lost pass or new valid CLI error. The 955 and 1,044
historical subsets retain 431 and 495 matches. All six raw-file controls and five
class/TIM source controls pass. No model entity-count drop occurs in the measured
before/after diagnostics. The 32 panic entries are distinct source variants of
two preserved sequence underflow signatures (24 with len2, eight with len1), not
32 independent regressions. The unchanged class `body_ownership` parse error is
also retained.

The independent fresh matrix has **29 inputs: 14 valid, 15 negative**. Eleven valid
cases match both exact source and actual CLI SVG. Three valid mismatches are the
named-key regression, raw macro prefix, and adjacent HslColor contract. Seven
negative inputs are rejected; the others remain accepted by Rust. Rejection alone
is insufficient where first-error ordering differs. Every intended classification
in the immutable request is kept separately from the observed classification:
`sub_suffix_continuation`, `keys_nested_call`, and `adjacent_dispatch` turned out
Java-invalid and were never counted as positive matches.

## Gates and limitations

Both main tiers ran **once** on the exact candidate:

- Product: **11,179/11,251**, exit 101.
- Assisted: **10,497 passed, 754 failed, 1,299 skipped**, exit 101.
- Parser: **597 passed**, exit 0. CLI pipe: **15 passed**, exit 0.
- Original standing perturbations: **562/1,547**, exit 101; matching-environment
  actual CLI: **588/1,547**. All 26 cwd/timezone/environment differences are listed
  in `analysis.json`. None is waived.
- Guards: **4 passed, 2 failed**, exit 101 (decimal ratchet and review evidence).
- Owned rustfmt and diff check: exit 0. Clippy: exit 101 at unchanged
  `parse/component.rs:72` dead code.

After adding the 29 immutable pairs and six fresh review records, only the
standing/guard evidence checks were repeated: **572/1,576**, exit 101; guards
**4/6**, exit 101. The fresh external-include control additionally needs its
specified cwd; it passes actual CLI with that cwd. Error-page and empty non-XML
references remain red in the unchanged standing oracle even when Rust correctly
rejects them. Main tiers were not repeated. The final staged diff check exits 2 solely for eleven
raw comparator stdout files ending in blank lines; those exact captured bytes
are preserved, with diagnostics in `final-diff-check.txt`. No baseline, comparator, skip rule,
error census, reader policy, date code, parser handoff, renderer, or target changed.

Initial submodule initialization returned exit 1 because Git disallowed the
configured local-file transport. A command-scoped file-transport allowance then
initialized this checkout's submodule at the exact pinned gitlink; no parent
checkout was modified. Both fresh CLI builds completed with exit 0. All substantive
nonzero gate/replay statuses remain in their command records, including negative
comparisons and the known panics; none is converted into a green test.

## Reference and archival provenance

No local Java/javac, SSH retry, new remote agent or shipping action occurred.
The parent relayed immutable requests to the existing Monkey Man runner:

- TIM24: 24 cases/27 resources, worker `worker.SSQHpgKU`; returned gzip SHA256
  `ad5c57f022f0740737b32bc117a205941f15419e3af561b9fd2a03b9e9325f82`.
- Supplement5: five cases/six resources, worker `worker.NCa5TzJC`; returned gzip
  SHA256 `08e901e88fe5e116f94583e12e9c25e0665e681a9ae3bd0d94bd67f611f7242f`.

Requests, manifests, source/include/probe hashes, resources, per-case cwd, GMT+11:00,
environment and actual wrapper/Java argv were independently checked. All 58 calls
retain raw stdout/stderr/status, no timeout, unchanged SourceProbe.java, UTF-8,
en_AU, headless AWT and pinned JAR SHA256
`aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.
XML validity, error-page classification, process status and probe status are
separate. Remote Java21.0.5/dot12.2 differ from historical Java21.0.13/dot16.1;
this is no historical binary/font/Graphviz equivalence claim.

Full raw returned payloads and transports are under
`test-diagrams/perturbations/tim-builtin-value-boundaries/checker5/relay/`.
Every fresh input has an exact same-stem Java SVG under its mechanism's `checker5/`
directory, including empty and error outputs. Adjacent `evidence/` directories
retain raw remote streams, commands and actual before/after CLI/model captures.
The archive contains all frozen-cohort captures and gate logs. All **6,967** old
tracked evidence/fixture files match the import Git blobs; all **3,094** original
pair files match the frozen size/SHA256 membership. No old record was rewritten.

Reproduction scripts use the private cache
`/Users/marcelo/Library/Caches/rustuml-review-1703a272`; restore the archived files
there and build each exact revision into its distinct target. `replay.py` reads
only fixed checked-in memberships and the unchanged comparator probe. Do not rerun
immutable generators into an existing input directory. `gates.py` is the original
exact-candidate run; use its log instead of repeating the main tiers to obtain a
preferred result. The raw-file blank-line policy stays owner-approved; historical
GET references are untouched. Broader macro-body semantics, Unicode/locale,
renderer geometry and date dependencies remain unresolved.
