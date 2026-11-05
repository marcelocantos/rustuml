# TIM backslash line continuation (🎯T14.22), round 1: independent checker review

Checker: Claude Opus 5.5 (`claude-opus-5-5`), branch `review/tim-continuation-claude-v2`, replacing the stalled checker. The stalled checker's cache was reused only after re-verification: its prepared fresh cohorts were re-hashed against the relay references (`verify-fresh.json`), and its binaries were **not** reused (all four trees rebuilt from `git archive`; `binaries.sha256`). Records: `../review.json` and the whole-line addendum `../../sprite-command-post-tim-ownership/review-5.json`. The maker record (`../README.md`, `../maker-results/`) is preserved.

**Verdict: rejected.**

## Cause

`tc1_seq_indented_end_after_backslash` (BU, Java-valid, held out) goes from mismatch at every before to a CLI error at `594c2b3c5`:

```
@startuml
Kea -> Tui : last \
  @enduml
```

- Java renders the message `last   @enduml` at line 1. `Pipe#readSingleDiagram` ends the input only on a line that `startsWith("@enduml")` at column 0 (Pipe.java:203), so it appends `@enduml` (line 219). `ReadFilterMergeLines` merges the indented end into the message, and the appended end closes the block.
- Rust's `merge_ending_backslash_lines` decides whether an end was appended with `is_end_directive`, which skips leading whitespace and also accepts `\end`. It sees an end, appends none, the merge swallows it, and `parse_with_base` reports "No valid @start/@end found".
- The account states the column-0 rule; the code does not follow it. The fix belongs in this packet.

## Other losses

| Case | Cohort | Change | Class | Owner |
|---|---|---|---|---|
| `mc1_seq_foreach_body` | BD | lost rejection vs `93bf53e18`, `2a9373aca` | accidental-rejection residue: Java fails in `JsonParser` (`EaterForeach` → `Json.parse` on `[Ash, Elm]`); Rust accepts the list at every build and rejected only the unjoined `tall` | none: proposed target |
| `ck4_procedure_sprite_end_comment_two`, `mk4_tim_sprite_in_procedure` | BA, AY | lost match vs `93bf53e18` | accidental-match residue, unchanged | T14.33 |
| `ck3_svg_inline_quoted_apostrophe_seq` | AX | lost rejection vs `93bf53e18` | accidental-rejection residue, unchanged | T14.32 |

Against `9834a7194` there are no lost matches or rejections. New valid-input errors there: the tc1 case above; the 13 approved T14.21 retained sources; and 11 fresh sources outside the approved set (10 T14.21 misdispatch, `ck4_seq_divider_trailing` T14.31 comment truncation), all present since `93bf53e18`.

## Numbers (after `594c2b3c5`, unchanged `compare_cli` and scripts, 4 modes)

| Cohort | n (Java-valid) | after match | vs `2a9373aca` | vs `93bf53e18` | vs `9834a7194` |
|---|---:|---:|---|---|---|
| BU (tc1) | 40 (37) | 25 | +23 gains, 2 newly rejected, **1 new valid error** | same | same |
| BD (mc1) | 33 (29) | 23 | +18, 2 newly rejected, 1 lost rejection | same | +18, 2 newly rejected, 1 new valid error (T14.21) |
| BA | 64 | 41 | +6 | +37, 4 newly rejected, 1 lost match (T14.33) | +36, 2 new valid errors |
| AY | 48 | 34 | none | +21, 1 lost match (T14.33) | +20 |
| AZ | 8 | 3 | none | +3 | +3, 1 new valid error |
| AX | 77 | 26 | none | +18, 1 lost rejection (T14.32) | +14, 2 new valid errors |
| AV | 63 | 20 | none | +10, 3 newly rejected | +8, 2 new valid errors |
| AT | 45 | 19 | none | +7, 1 newly rejected | +4, 1 new valid error |
| AS/AQ/790/28 | 915 | 247 | none (0 bytes) | +59, 35 newly rejected | +53, 2 new valid errors |
| retained | 4,174 | 2,667 | +2 (exception sources) | +22, 2 newly rejected | +78, 13 new valid errors (approved) |
| golden | 12,550 | 11,181 | 0 outcome changes, 12 bytes (T14.17) | 1 newly rejected | 126 newly rejected |

No panics except the unchanged `--yaml` set; no timeouts.

Gates: lib 1,507/0; pipe_mode 13; no-oracle 11,181/11,251 and golden_pairs 754 failures, names byte-identical to `9834a7194`; parity_perturbations 0 new in pre-existing directories, 8 fixed, new directory 46/53; harness_guards: frozen-item guards pass; decimal ratchet same 5 render files; records add only the T14.6 rejected-review categories (`gates/gv-post.txt`).

## Files

- `summary.json`, `rejection/`, `runs/` (fresh cohorts), `manifests/`: measurements.
- `controls.json`: Rust-only attribution controls.
- `probes/`: per-case Java vs Rust extracts for BU and BD, and the stalled checker's pre-Java Rust predictions.
- `request-tc1/`: the BU request and results as relayed (SHA256SUMS).
- `registered-pairs.json` (53, under `test-diagrams/perturbations/tim-line-continuation/`), `evidence-only-cases.json` (20).
- `support/`: every script used; unchanged scripts' hashes in `unchanged-checker-scripts.sha256`.
