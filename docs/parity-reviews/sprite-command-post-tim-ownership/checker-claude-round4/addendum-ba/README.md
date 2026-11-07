# Round-4 checker addendum: batch BA (Java references for ck4)

This addendum completes the round-4 checker review of maker tip `c9e11493c`. The record is `../../review-4.json`. `../../review-3.json` and `../README.md` are preserved unchanged.

**Final verdict: rejected.**

## Reference identity

- Location: `~/Library/Caches/rustuml-relay-parent/sprite-round4-ck4/`, relayed by the parent from the pinned wrapper (worker `scratch/worker.QsM9mjbH`).
- The manifest and input bundle are byte-identical to the request staged in `../request-ck4/`.
- Hashes are in `reference-ba/SHA256SUMS`; `reference-results.json` is `5587f87a…`.
- `prepare.py` (unchanged) verified every source, SVG and stderr hash.
- Validity comes from the raw SVG plus stderr markers.

**Java outcome:** 57 cases exit 0 and 7 exit 200; none timed out. By markers, 57 are valid and 7 invalid.

**Disagreements with intent** (recorded; intent not rewritten). These three were intended valid but are Java syntax errors (`Syntax Error? (Assumed diagram type: sequence)`):
- `ck4_nwdiag_trailing`
- `ck4_salt_trailing`
- `ck4_seq_span_in_quotes_trailing`

## BA results (after `c9e11493c`; 4 modes; unchanged `compare_cli`)

| Against | Gains | Newly rejected | Lost matches | New valid-input errors | Lost rejections | Panics / timeouts |
|---|---:|---:|---|---|---:|---|
| `93bf53e18` | 31 | 4 | `ck4_procedure_sprite_end_comment_two` | `ck4_seq_continuation_next_leading`, `ck4_seq_continuation_comment_only_then_msg` | 0 | 0 / 0 |
| `044f73657` | 31 | 4 | none | same two | 0 | 0 / 0 |

- Gains are 28 mismatch→match and 3 valid-error→match.
- `ck4_tim_include_trailing` goes from valid-error to mismatch.
- Rejection contract (unchanged script): 7 entries, rejected 0 → 4 against either before, no introduced losses.
- Rust accepts `ck4_nwdiag_trailing`, `ck4_salt_trailing` and `ck4_inv_include_missing_after_comment` in all three builds. Java says `cannot include ck4-missing.iuml` for the last. These false accepts predate the packet.

## Packet-caused losses

### 1. Real new valid-input errors (round 4; against both befores)

`ck4_seq_continuation_comment_only_then_msg`:
```
/' c '/ \
A -> B : hi
```

`ck4_seq_continuation_next_leading`:
```
A -> B : one \
/' c '/ two
```

What Java does (confirmed):
- It merges each pair into one message before TIM: `hi` and `one  two`. Both get `data-source-line="1"`, the location of the merged first line.
- The merge happens in `BlockUmlBuilder` → `preproc2.Preprocessor` → `ReadFilterMergeLines`. The continued line is read through `ReadFilterQuoteComment`, which calls `removeInnerComment`.

What Rust does:
- No Rust build merges continuation lines (🎯T14.22): `A -> B : one \` / `two` errors in all three.
- Before the packet, Rust dropped the whole `/' c '/ …` line as a comment. Its output was wrong (mismatch: line numbers, message text) but rendered.
- `c9e11493c` classifies the line as PLAIN, as Java's `TLineType` does, and keeps its tail. Without the merge, that tail is a standalone ` \` or ` two` line that no sequence command accepts, so the CLI exits 1.

Classification: a genuine regression from mismatch to valid-input error, caused by the round-4 change through its interaction with the missing 🎯T14.22 port. It is not accidental-match residue, because neither before matched.

Settlement against 🎯T14.22:
- The faithful fix is the `ReadFilterMergeLines` / `ReadFilterQuoteComment` port, before TIM.
- Both inputs belong in 🎯T14.22's held-out acceptance set.
- Until 🎯T14.22 lands, the packet cannot avoid these two errors without a Rust-only workaround, such as restoring the over-broad whole-line drop. A workaround like that would move away from Java elsewhere.
- So the packet either waits for 🎯T14.22, or the owner records them as an approved exception.

### 2. Accidental-match residue (against `93bf53e18` only; 🎯T14.33)

`ck4_procedure_sprite_end_comment_two` goes from match to mismatch: Java puts `Elm` at 8 and `Oak` at 9; `c9e11493c` gives 11 and 12. This is the same mechanism as the review-3 AY loss `mk4_tim_sprite_in_procedure`.

BA confirms 🎯T14.33 with Java controls:
- `ck4_ctl_procedure_four_classes`: Java puts `Oak` at 8; all three Rust builds give 11.
- `ck4_ctl_procedure_sprite_nocomment`: Java gives `Oak` 8 and `Elm` 9.

`93bf53e18` matched only because it never expanded the procedure.

### 3. Accidental-rejection residue (from review-3, unchanged)

`ck3_svg_inline_quoted_apostrophe_seq` (round 3) is tracked by 🎯T14.32. BA adds no lost rejection.

## Cross-family result

Outside the two continuation cases, the port matches Java or improves on the pre-packet output in every family probed:
- **Fixed cases (31):** sequence (apostrophes, URL, creole, quotes, `/'''k'''/` in a quoted label and in a participant name, notes, CRLF, divider, title, skinparam blocks), class, component, state, activity3, object, YAML, JSON, mindmap, timing, EBNF, and `!function`, `!define`, `!definelong`, `!while`, `!foreach` and `!if`.
- **Mismatches in all three builds, so not caused by the packet:** WBS, gantt, the double-pass procedure and sub probes, the non-sequence `\` cases, and the procedure source-line controls.

## Pairs (T14.6.7)

- 54 Java-valid BA pairs are registered in `test-diagrams/perturbations/sprite-command-post-tim-ownership/`: 35 pass, 19 fail.
- Evidence-only (`evidence-only-cases.json`): 7 Java-invalid cases, plus 3 cases that need a sibling resource (`!include` / `!includesub`).
- `parity_perturbations` rerun after registering: 2,533 failures. The 19 new ones are exactly the pairs the CLI fails (harness = CLI), and no earlier pair changed.
- `harness_guards` with `review-4.json` in place: `gates/harness_guards-post-ba.log.gz`.

## Next obligations (bounded)

1. **🎯T14.22:** port the pre-TIM `\` merge (`ReadFilterMergeLines`, with `ReadFilterQuoteComment#removeInnerComment` on the continued line). Acceptance: both BA continuation cases, the two retained continuation sources, and held-out macro and theme variants match, with no pass lost. Then re-review this packet on top of it, or the owner records the two cases as an approved exception.
2. **🎯T14.33:** `mk4_tim_sprite_in_procedure`, `ck4_procedure_sprite_end_comment_two`, `ck4_ctl_procedure_four_classes`, `ck4_ctl_procedure_sprite_nocomment`, `bd087_macro_valid` and `c75_sprite_procedure_legacy` match.
3. **🎯T14.32:** decide on `ck3_svg_inline_quoted_apostrophe_seq` (parent or owner).
4. **🎯T14.17 inputs (false accepts in all builds):** `ck4_nwdiag_trailing`, `ck4_salt_trailing`, and the missing-`!include` case `ck4_inv_include_missing_after_comment`.
