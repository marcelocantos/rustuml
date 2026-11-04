# Independent review of class packet8

All three new claims are **rejected**, with bounded support for their repaired behavior. This reviews exact candidate `68ab512be964e243ae1943f7a9fefd3537091401`, production `90eab708a25bf80b86c4e44b571aa79e3f9b2d05`, against `154d4c7fd58bc7a1b91df7d01f530b3ea911a5fc`. The checker worked alone, implemented none of the candidate and changed only new evidence. Read [audit.md](audit.md) for causal findings and repair guidance.

- [Paint inventory review](../review.json): four of eight fresh cases match; root/local/nested and group-note whole geometry still fails despite correct metadata and order.
- [Consuming note grammar review](../../class-attached-note-consuming-url-grammar/review.json): seven matches, three valid-input parser failures, four Java-error controls. Java cleans `«Vessel»`, `(Vessel)` and `[Vessel]`; Rust still rejects all three targets.
- [Shadow consumer review](../../class-normalized-shadow-style-consumer/review.json): ten of eleven match; the empty-package path still lacks Java's shadow filter even when the native-class shadow and canvas match.

All 33 new source/reference pairs, including every failure and negative, are registered under the corresponding `test-diagrams/perturbations/<mechanism>/checker-112250ad/` directory. They remain executable through the existing standing harness. Java accepts 29 class diagrams and emits four error SVGs; Rust correctly rejects those four negatives. No welcome, empty or malformed XML output occurred. `shadow_invalid_digits` was intended negative but is Java-valid because `ValueImpl` falls back to 1.5. Its original name, bytes and classification intent remain preserved alongside the observation.

## Real CLI reproduction

The independent release CLI renders from source without oracle input. The unchanged comparator operates afterward. Every before/after status, CLI exit, SVG hash and YAML hash in the immutable 1,851-entry maker ledger is reproduced exactly. Full commands, outputs and inventories are in the archive; [summary.json](summary.json) contains the totals.

| Cohort | Entries / unique sources | Before matches | Candidate matches | Candidate mismatches / errors / invalid references |
|---|---:|---:|---:|---:|
| Frozen1826 | 1826 / 1752 | 1103 | 1121 | 563 / 60 / 82 |
| Maker fresh25 | 25 / 25 | 4 | 21 | 0 / 0 / 4 |
| Combined maker ledger | 1851 / 1777 | 1107 | 1142 | 563 / 60 / 86 |
| Independent checker33 | 33 / 33 | 4 | 21 | 5 / 3 / 4 |

There are 74 extra duplicate entries in the frozen inventory; none is silently removed. New checker33 is disjoint, so the full reviewed union is 1,884 entries and 1,810 unique sources, still with 74 duplicates. [supplemental-audit.json](supplemental-audit.json) gives the original eight-way frozen partition, exact membership and duplicate groups. The first 810 entries are not claimed to be the original810 census.

Frozen YAML/status is byte-unchanged for 1,824/1,826. The two changes remain the quoted multiline URL correction and recovered curly note target. No previous valid class UID stream changes, counted model drops, lost passes or new valid-input errors occur. The existing 48 current valid review/maker DOM inventories independently agree, including inherited/direct URL owners and metadata order; that does not waive group geometry. Of the 82 frozen invalid references, 31 are still accepted by Rust and remain failures, not positive evidence.

Standalone before binary SHA256: `dbab4d2d64928c321939c930969b10f0a769015e9e23bc17dc8f479a663f1d6b`.
Standalone candidate binary SHA256: `652871706304472161bbf8d508772820443848207ffd78458e028983a8ca539c`.
Workspace-test-built candidate binary SHA256: `02a859a947acc08924eace1c3a2ba944ed220ead4ecfd7ddafcc6f9192960a22`.

The standalone executable remains preserved in the private cache. Cargo's later workspace CLI tests produced a second executable; its real SVG/YAML outputs were compared against all 1,884 earlier captures with zero differences. This final executable check repeats no build, broad gate or Java generation. Actual recorded maker before/draft/final binary hashes were also independently verified, without treating those executables as substitutes for the independent builds.

## Broad checks and immutable history

Required broad checks ran once on the clean exact candidate, before registration of new pairs/reviews. No broad check was repeated for packaging. See [gates.json](gates.json) and raw archive logs.

- Units: 1,456 pass. CLI/error/docs: 34 pass. Formatting: pass.
- Library no-oracle diagnostic: 11,189/11,251, exit 101. Assisted: 10,525/11,251 eligible, exit 101 (12,550 total, 1,299 skipped, 726 failures). Failure-name files are identical to the maker's candidate files, with no new failure names versus preserved before files.
- Standing exact-candidate perturbations: 1,028/1,718; 690 failures include 76 invalid references, 33 panics and 27 parse failures; exit 101. This run predates checker33 registration. No post-registration all-suite pass is claimed.
- Guards: 4/6, exit 101. Clippy: 20 diagnostics, exit 101, with the same categories/counts as the maker.

These library diagnostics are separate from real-CLI truth. The red guards, standing cases and whole-case failures prevent acceptance. Registering evidence does not change that result.

[integrity.json](integrity.json) validates all 9,254 historical protected blobs, the identical59d50dbc→c51bb768 import patch, 295 maker-package files, all 34,246 archived members, source/reference snapshot hashes and actual maker executable hashes. Failed draft compile/test logs remain intact. Artifact counts show preservation, not acceptance.

The three original account commits precede committed production. Git cannot prove no earlier uncommitted draft. Three inaccurate original location qualifiers were corrected only postimplementation; their corrected paths/hashes are verified, but originals and chronology remain unchanged. Earlier missing-mechanism fields, after-draft corrective accounts and rejected verdicts remain immutable and unexcused.

## Reference provenance and reproduction

Only the parent's existing Monkey Man runner generated the fresh references. Both batches used `/Users/marcelo/rustuml-reference/bin/reference --cwd PRIVATE_CWD --timezone GMT+11:00 -- -pipe -tsvg`, with the placeholder substituted only for the private worker directory. Resources are empty. Source bytes/SHA/base64, exact membership, raw streams and hashes, wrapper/Java argv, cwd, timezone, headless flags, environment deltas, exits and no-timeout records were independently verified.

Original30 input gzip SHA256: `8063be11519441219e169e562e5f90c5389983b75a1d97fdc2a578e5522ec3db`; returned gzip: `8c1a631eb921b899d55c808c83b6572024a51c1f5e10e72db875c1ddb1a358ee`.
Supplement3 input gzip SHA256: `99244721b15edb563c8b335d7d296a12f65002d38f03be4219ad55539331ea30`; returned gzip: `d983d4cd8c89d9b5511e7370b1706a6ce54c723fc70b5fc1e0e429092914708f`.

Pinned Java: `71806a23780b04a5ccde2f8ceb5121edad5eb711`. JAR: `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`. Wrapper: `6115a3503877999796cb98b9efb6872d9bb8090c2f7b986706919e475dee069e`. Historical fonts/Graphviz environment equivalence remains unproved; source-level causal explanations do not assume it.

[evidence.tar.gz](evidence.tar.gz) holds fresh inputs/references, both complete CLI/YAML replays, final-binary captures, DOM/model inventories, frozen content-addressed source/reference snapshots, scripts, manifests, raw relay bundles, build logs and gate logs. [archive-integrity.json](archive-integrity.json) verifies all members and the archive digest. Executables/build targets are excluded; exact revisions and hashes are retained. The existing immutable maker archive remains the dependency for historical draft logs and original ledger inputs: `../maker-evidence/packet8/packet8-evidence.tar.gz`, SHA256 `8fb7eb16f32bc0fea14a891ab8e5b84744e1fa7e5268cc38c0b596f60943f072`.

To reproduce, extract into a new private cache, retain immutable manifests, and explicitly remap recorded absolute source/build/capture paths. Historical CLI runs use the preserved maker cwd and stdlib; do not trim sources or silently change include resolution. Rebuild exact Git revisions into separate private targets, then run the retained CLI/comparator scripts against the snapshots. Existing stored references suffice; replay scripts must not start Java. The source archive for the before build is a plain `git archive`, not a shell-created worktree.

Group-note305vs301, fonts/Q/cloud/header/native point-engine dependencies, rooted CommandLinkClass multipass, TIM source_line0, nested-quote/generic integration and other-family dispatch remain explicit handoffs in [audit.md](audit.md). No native/fault review retry, layout experiment, child, local JVM, SSH, shared-metric change, target edit, shipping or policy change was performed.
