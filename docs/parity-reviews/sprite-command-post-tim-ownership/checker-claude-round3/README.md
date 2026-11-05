# Post-TIM sprite command ownership, round 3: checker review

Checker: Claude Opus 5.5 (`claude-opus-5-5`), branch `review/sprite-round3-claude`. The record is `../review-2.json`. The earlier reviews (`../review.json`, `../checker-claude/`) are preserved.

**Verdict: rejected.** Packet tip `607890d0d`; befores `93bf53e18` (pre-packet) and `dea00b295` (round 2).

## Cause

Java strips a trailing `/' … '/` span from every line:
- `TContext#buildCodeIterator` applies `CodeIteratorInnerComment`, which calls `StringLocated#removeInnerComment`.

Rust has no port of `removeInnerComment`. The round-3 comment exemption skips all Rust comment handling on sprite-owned lines, so the sprite command sees the comment.

On the checker batch AX (Java-valid by raw SVG plus stderr), against `93bf53e18`:
- **4 lost matches that are now exit 1:**
  - `ck3_pixel_end_inner_comment_class`
  - `ck3_svg_end_inner_comment_nospace_component`
  - `ck3_svg_inline_inner_comment_object`
  - `ck3_pixel_start_inner_comment_deployment`
- **2 new valid-input errors:**
  - `ck3_svg_end_inner_comment_state`
  - `ck3_pixel_end_inner_comment_seq`
- **1 lost rejection:** `ck3_svg_inline_quoted_apostrophe_seq`. Java shows an internal-exception page.

All seven were already wrong in round 2; round 3 did not repair them.

## Counts (after = `607890d0d`)

| Cohort | n | Losses vs `93bf53e18` | Losses vs `dea00b295` | Gains vs `93bf53e18` |
|---|---:|---|---|---:|
| AV (maker-held-out mk3) | 63 | 0 | 0 | 10 (+3 newly rejected) |
| AX (checker ck3) | 77 | 4 lost matches, 2 new valid errors, 1 lost rejection | 1 lost rejection (`ck3_usecase_business_actor_type1`) | 17 |
| AT | 45 | 0 | 0 | 7 (+1 newly rejected) |
| AS/AQ/790/28 | 915 | 0 | 0 | 59 (+35 newly rejected) |
| retained | 4,174 | 0 | 0 | 19 (+2 newly rejected) |
| goldens | 12,550 | 0 | 0 | 0 (+1 newly rejected) |

Panics and timeouts: 0, except `--yaml` panics on the same 148 golden and 63 retained sources in all three builds.

Details: `summary.json`, `runs/*-changes-*.json`, `runs/rejection-*.json`.

## Files

- `request-ck3/`: the staged Java request.
- `reference-ax/`: the parent-relayed Java results (hashes in `../review-2.json` commands).
- `fresh-*-manifest.json`: per-case Java validity.
- `runs/`: the three builds' replays of every cohort.
- `gates/`: failure lists and logs. The `*-post` files are after registering pairs.
- `support/`: the scripts used.
- `ax-resources/`: sibling files for the evidence-only include and theme cases. `puml-theme-ck3.puml` is stored as `.txt` so the perturbation walk does not collect it.
- `registered-pairs.json` and `evidence-only-cases.json`: which held-out cases are executable pairs under `test-diagrams/perturbations/sprite-command-post-tim-ownership/`.
