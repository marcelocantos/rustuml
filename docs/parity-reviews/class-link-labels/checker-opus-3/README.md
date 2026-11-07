# Class link labels: integration review 3 (Opus)

This is an independent checker review of the integration commit `a40e77c91`, the head of
`integ/g4-links`. It brings `02e4fdad3` (G4 code tip `50218c805`) into parity-recovery
`d86ac8bef`, with goldens at `9efdc12f`. The record is `../review-3.json`.

**Verdict: accepted. Accept-ready for local merge into parity-recovery.**

`er/er_ecommerce_schema` is declared residue. It was already failing at both parents
(strict DIFF 99 at `d86ac8bef`, DIFF 100 at the G4 tip).

## What the parent changed

- **Commit shape.** `a40e77c91` has one parent, `d86ac8bef`: it is a squash of the merge.
  `git merge-tree --write-tree d86ac8bef 02e4fdad3` is clean, giving tree `24b476ab9`, and
  since merge base `9fb2e52a6` no file is touched by both sides. Every touched file is
  byte-identical to one side's version, except the two clippy edits.
- **Clippy fix (behaviour-preserving).**
  - In `protect_creole_links`, `repeat(..).take(full.len())` became
    `repeat_n(.., full.len())`. It still pushes `full.len()` copies of U+E000.
  - In `restore_creole_links`, `repeat(..).take(token.len()).collect()` became
    `"\u{E000}".repeat(token.len())`. The literal is one char, so the placeholder is still
    `token.len()` chars.
  - `PlacedRow` aliases the identical tuple type.
  - Evidence: the CLI output is byte-identical to the G4 tip on all 12,550 goldens and all
    243 G4 cohort cases. Clippy per crate is identical to `d86ac8bef` (parser 1, render 20,
    layout 1, rustuml 0, oracle 1).

## Counts (CLI `-pipe -tsvg`, vendored `compare_svg_strict`, Java-valid only)

| Cohort | d86ac8bef | G4 tip | Merge | Must equal | Equal bytes |
|---|---:|---:|---:|---|---|
| DA `grok-g4-recheck-heldout` | 3/55 | 25/55 | 25/55 | G4 tip | yes |
| CO `grok-g4-checker-heldout` | 13/94 | 75/94 | 75/94 | G4 tip | yes |
| CK `grok-g4-class-link` | 1/13 | 13/13 | 13/13 | G4 tip | yes |
| CT `grok-g4-ct` | 0/46 | 45/46 | 45/46 | G4 tip | yes |
| CY `grok-g4-cy` | 0/4 | 4/4 | 4/4 | G4 tip | yes |
| G1 `grok-g1-checker-heldout` | 37/41 | 37/41 | 37/41 | d86 | yes |
| G6 `grok-g6-creole` | 13/17 | 1/17 | 13/17 | d86 | yes |
| G6 `grok-g6-checker-heldout` | 15/40 | 4/40 | 15/40 | d86 | yes |
| G6 `grok-g6-cs` | 25/25 | 3/25 | 25/25 | d86 | yes |
| G6 `grok-g6-recheck-heldout` | 7/13 | 2/13 | 7/13 | d86 | yes* |
| G2 `grok-g2-checker-heldout` | 0/28 | 0/28 | 0/28 | d86 | yes |

\* Two `g6r3_invalid_utf8_*` inputs panic at `main.rs:91` on every binary. This is
pre-existing, and their stderr differs only in the thread id.

Java-invalid cases are excluded: exit 100 or 200, or an exit-0 "has crashed" page with a
Java stack trace on stderr.

- **Goldens (12,550):** strict MATCH is 11,179, 11,180 and 11,180 (d86, G4 tip, merge).
  - Against d86, 0 are lost and 6 change bytes: `splines_multilevel_inheritance` is a gain,
    `splines_self_reference_ortho` goes from 10 to 1, `er_financial`, `er_healthcare` and
    `er_hr` keep their counts, and `er_ecommerce_schema` goes from 99 to 100.
  - Against the G4 tip, 0 change bytes.
- **Perturbations (CLI, 6,408):** 0 lost against either parent, and no case differs from
  both parents. All 29 class-link-labels pairs pass.
- **Gate failure names:**
  - `golden_no_oracle`: 70 at d86, 69 at the merge. The only difference is the removal of
    `splines_multilevel_inheritance`.
  - `golden_pairs`: identical names on both, 10,497 passed and 754 failed.
  - `parity_perturbations`: 2,662 failures at d86, 2,643 at the merge. 0 names are new.

## Gates on the merge tree

| Gate | Result |
|---|---|
| `golden_no_oracle` | pass |
| `golden_pairs` | fails identically on both trees (pre-existing) |
| `parity_perturbations` | fails as at d86, with 19 fewer failures (pre-existing) |
| `pipe_mode` | 17/17 |
| `doc_claims` | 2/2 |
| workspace lib | all pass |
| clippy | identical to d86 |

`harness_guards` by category:

- Five categories pass on both trees.
- `decimal_literal_ratchet` fails identically on both trees.
- `parity_review_records_are_complete`: with this review-3, its messages are identical to
  d86's, 2,684 lines with 0 for class-link-labels.

## Residue

- **`er_ecommerce_schema`.** The xlabel for the `parent of` self-loop is placed at x 1027.19,
  where Java has 753.38, and it widens the canvas to 1105 (Java 1035). The reason is that
  Rust routes the ortho self-loop round the right of `Category` and Java routes it round the
  left. A target is proposed for the route.
- **Combined behaviour is unmeasured.** No Java evidence covers inputs that combine G4 link
  labels with G2 advances or G6 character references, such as a non-ASCII or `&#...;` link
  label in Helvetica. A held-out batch is proposed.

## Files

| File | Contents |
|---|---|
| `scripts/` | The sweep, render, score and cohort-compare scripts, and the scorer (vendored `compare.rs`, blob `139bbb52`, unchanged at 9fb2e52a6, d86, the G4 tip and the merge). |
| `cohorts/*-score.json`, `cohorts-summary.json` | Per-case verdicts and the summary. |
| `golden-sweep-changed.json`, `pert-sweep-changed.json` | Totals and the cases whose output differs between the binaries. |
| `no_oracle_names-*`, `pairs_names-*`, `pert-names-*` | Gate failure names for d86 and the merge. |
| `guards-records-*` | `parity_review_records_are_complete` messages, normalised. |
| `clippy/` | Per-crate diagnostics with line numbers stripped. |
