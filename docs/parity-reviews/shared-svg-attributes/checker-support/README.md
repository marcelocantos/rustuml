# Independent shared SVG serialization review

**Verdict: rejected.** Reviewer: new independent T3 task
`02b1a747-7512-424a-bf7a-dd53d71bb634`, GPT-6 Astra, high effort, Codex harness.
No implementation involvement, children, additional worktrees, production edits,
target edits, baseline changes, comparator changes, shipping or local Java.

Evidence tip: `a2167f93695497e49b8ae71c4830bac96309023b`.
Production: `a33dccc8e94dc44b5bafdc756fc1b88c42823a7d`.
Base: `b8905e296b8865e53f3c573abaebf4adb632d751`.
Java: `71806a23780b04a5ccde2f8ceb5121edad5eb711`, OpenJDK 21.0.5,
JAR SHA256 `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.

## Findings that prevent acceptance

1. The XML oracle's DOCTYPE guard can accept an unapproved internal DTD.
   It searches the first `<!DOCTYPE` substring, even inside a comment.
   A comment containing the approved declaration, followed by an actual
   internal-entity DTD, passes the unchanged test with its 26-count assertion.
   Scratch-only fault injection proves this; it is not a shipping-path claim.
   No external resource was resolved. See `dtd-mutation-results.json`.
2. Valid `comment_double_hyphen` produces malformed Rust SVG before and after:
   `<!--class Double--Tail--->`. Java's DOM serializer inserts spaces around
   consecutive/trailing comment dashes. The 45-site comment inventory's clean
   original corpus does not establish universal well-formedness.
3. Four fresh C0 sources are Java SAXException crash diagrams, with matching
   stderr and exit 0. Rust emits malformed SVG. They are outside the three
   approved source hashes and are not added to the exception list.
4. `shape_alpha` still emits a raw eight-digit state fill, whereas Java splits
   RGB and `fill-opacity="0.50196"`. The direct state writer only escapes it.
5. Font definition/consumer, URL grammar and nonshared text paths still differ.
   Nested quote trimming in component font selection loses a quote; several
   consumers ignore the family. Salt/JSON/NWDiag emit literal transparent-text
   markup. All whole-output failures remain failures, even when the shared
   helper behaves correctly.

The five records are `../review.json`, `../font-family/review.json`,
`../strict-xml/review.json`, `../../shared-svg-color/review.json`, and
`../../shared-svg-color/transparent-text/review.json`. Their bounded findings
are not acceptance attestations.

## Reproduced actual CLI measurements

| Membership | Inputs | Valid references | Before | After |
|---|---:|---:|---:|---:|
| Golden | 12550 | 11251 | 11179 | 11179 |
| Prior retained cohorts | 2082 | 1941 | 974 | 983 |
| Maker fresh | 22 | 18 | 4 | 10 |
| Independent fresh | 87 | 83 | 2 | 34 |

No lost passes, new process errors or new panics in any before/after replay.
Exactly 16 goldens change SVG bytes: 14 pass→pass and 2 fail→fail. Each has
identical decoded XML including comments, PIs, attributes, text and structure.
The fresh87 has 85 emitted outputs, five malformed, two process errors with
empty stdout (`family_salt`, `family_json`), and zero panics. All five malformed
outputs and both errors already occur before the packet. Four malformed cases
are C0 negatives; the fifth has a valid Java reference.

The maker's archived 2104 YAML outputs are byte-identical. Its nine golden
YAML changes are map-order-only. The independent rerun finds one retained
sprite map-order change and five golden DOT map-order changes; all six decode
identically with YAML tags, sequence order and scalar contents retained.
This is T14.6.6 nondeterminism, not a claim of byte identity on rerun.

## One broad gate pass on the frozen candidate

These ran before registering the 87 new checker pairs. No broad reruns.

- Renderer library: 796/798; the same plain-theme and source-shadow-ID failures.
- CLI: 16/16. Strict XML golden: 12550 inputs, 12470 emitted, 0 malformed,
  80 errors, 0 panics. Perturbation: 1966 inputs, 1857 emitted, 3 approved
  malformed, 109 errors, 32 panics. The 26 DOT declarations pass.
- Product library tier: 11179/11251, with baseline ratchet failures retained.
- Assisted library tier: 10497/11251; failure-name files byte-identical to the
  maker's 72 product and 754 assisted failures.
- Standing library perturbations: 851/1966, 1115 failures.
- Guards: 4/6. Clippy stops at the known unused parser function. Format residue
  remains; full logs are archived. These are separate from CLI product truth.

Both malformed-output ratchet directions were fault-tested with the exact
unchanged test source in a scratch corpus: unlisted malformed output fails;
a listed source made valid fails the hash and stale-malformed checks. The
scratch mutations were discarded. The separate DTD acceptance test fails to
detect an injected unapproved internal DTD, as described above.

## Scope and preservation

The 901-site list across 24 files is intact. Independent checks cover 868
nonowned routing sites, 534 changed nonowned macros, and manual review of 33
owned sites. Their template/dataflow checks, producer substitutions and the
approved state placeholder exception are recorded in `routing-audit.json`.
No geometry or text edits were found in routed nonowned sites; no double
escaping, repair pass or fixture-shaped logic was found. Sequence is unchanged.
The four primary sequence attribute groups in the handoff are accurate; warning
attributes and title text are additional helper uses, not extra routed sites.

Normal production dependencies are byte-identical. Cargo.lock adds exactly
seven nodes and two dev edges. All 11031 old evidence/fixture files and the
imported rejection have identical Git blobs. All eight maker archives and
272037 entries verify on read-back. The identity-domain census independently
reproduces all six attribute counts across 14494 captures with no violations.
T14.15/T14.16/T14.20 are absent at the evidence tip, so their definitions were
read from T3's starting revision `d10f91cc9db68775a55859513f9bae8892810dd9`.
No targets were edited.

## Raw evidence and replay

`measurements.tar.gz` contains raw before/after CLI SVG, YAML, stderr and
comparison results, all fresh Java streams and original relay envelopes,
commands/scripts, logs and per-input outcomes. `measurement-archive.json` and
`measurement-entries.json.gz` record hashes and read-back verification.
Extract only to a private nonsynced cache. It contains no binaries or Java
runner; build the exact revisions with private target directories and jobs 4.
The scripts document execution, rather than authorizing another broad run.

Fresh Java used only the parent's existing Monkey Man runner, private worker
`/Users/marcelo/rustuml-reference/scratch/worker.L9dFp3l5`, cwd `new`,
GMT+11:00, env overrides `{}`. All sources and streams were hash-checked by
parent and checker. Java argv is derived from the pinned wrapper, not traced.
The initial golden hydration failure (`transport 'file' not allowed`) and
successful per-command local-transport retry are preserved.

Next obligations are bounded: close the DTD oracle acceptance hole; address
comment serialization and fallible C0 export; propagate colour paint roles and
font definitions to missing consumers; then recheck whole outputs. Existing
automatic, logical-font/metrics, gradients, base-style OR, warnings, stripes,
plain/code, unconsumed-line and sequence obligations remain explicit failures.
No combined correctness claim is made for the concurrent sequence packet.

The staged diff check reports intentional trailing spaces in five font-family
inputs. These are the padded-name perturbation axis and are preserved byte-exact
with their Java references; see `staged-diff-check.txt`. No code whitespace was
changed.
