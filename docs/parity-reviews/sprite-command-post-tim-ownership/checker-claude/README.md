# Post-TIM sprite command ownership: independent review, rejected

Reviewer: Claude Opus 5.5 (`claude-opus-5-5`), Claude Code in T3 Code, branch `review/sprite-ownership-claude`. No production changes, no local Java, no child agents, no push. Full record: `review.json`. Guard-schema copy: `../review.json`. Identities: `bindings.json`.

- **Packet:** maker tip `5cf1eac12`. Production commits are `95cf85252` (sprite) and `ecbd75ec8`/`87bdb0feb` (participant, reviewed in `sequence-participant-silent-residue/checker-claude/`).
- **Builds:** the before tree is `93bf53e18` and the after tree `5cf1eac12`, each a private `git archive` export built with `-j 6`.
- **Java:** `71806a23`, JAR `aa5262a2…`. Goldens at `9efdc12f`.

## Verdict

**Rejected.** On the checker's held-out batch AT, the packet causes 21 new valid-input errors, 5 lost whole-output matches and 2 lost rejections. AT has 45 Java references (42 Java-valid), staged as `request-batch1/` and relayed by the parent from `…/rustuml-relay-parent/sprite-ownership-checker-batch1/` (results SHA256 `98671af0…`).

| Cohort | n | Before (match / mismatch / valid error / rejected / accepted-invalid) | After | Losses |
|---|---:|---|---|---|
| AT (checker, held out) | 45 | 12/28/1/2/1 (+1 compare-101) | 10/9/22/0/3 (+1) | 6 lost matches (5 sprite, 1 participant), 21 new valid errors, 2 lost rejections |
| AS (maker batch2, held out) | 54 | 8/15/10/2/19 | 17/14/2/17/4 | 0 |
| AQ | 43 | 4/22/8/4/5 | 31/3/0/8/1 | 0 |
| 790 | 790 | 176/331/6/261/16 | 191/322/0/273/4 | 0 |
| 28 | 28 | 0/14/8/2/4 | 8/14/0/6/0 | 0 |
| retained | 4,174 | 2645/1076/17/368/68 | 2658/1063/17/370/66 | 0 |
| goldens | 12,550 | 11181/70/0/201/998 (+100) | 11181/70/0/202/997 (+100) | 0; 1 SVG byte change (`edge_render_sprite`) |

## Causes (Java trace in `review.json`)

1. **Multiline command bodies reach the sprite group.** This causes 17 new valid-input errors.
   - Java's `CommonCommands#addCommonCommands1` calls `addTitleCommands` (title, legend, header, footer multilines) before `addCommonCommands2` (sprites). Description `[ … ]` bodies and legacy-activity note bodies are also owned through END.
   - The Rust component, deployment, usecase, state, object and legacy activity parsers do not hold those bodies before `sprite::dispatch`. Body text such as `Sprite sheets are loaded` is therefore executed as `CommandSpriteFile`, which fails with `Cannot read`.
2. **Rust-only trailing-apostrophe stripping now reaches SVG sprite body and END lines.** Only the start line is exempt. This causes 4 lost matches that become errors, and 2 lost rejections.
3. **A pixel sprite inside a same-file `!startsub` is no longer registered.** Rust's TIM does not emit the startsub body; the old in-TIM collector masked that. This causes 1 lost match. The control without `!startsub` still matches.

## What holds

- TIM ordering is now correct: every error from the rejection is fixed.
- Unclosed blocks are rejected with Java's error contracts: `Cannot read: [2x2/16] {` for pixel headers and a syntax error for SVG headers.
- The unchanged `rejection_contract.py` on 313 Java-invalid inputs gives 269 rejected before and 304 after, with 0 lost. It exits 1 on 9 remaining false accepts. AT's 2 lost rejections are counted separately because that manifest predates AT.
- Model changes are explained (`runs/*-model-changes.json`).
- Gates, each run once:
  - no-oracle and `golden_pairs` failure names are identical to `9834a7194` (70 and 754).
  - `parity_perturbations` failures go from 2,396 to 2,366, with 0 new.
  - Guards are unchanged apart from the two pre-review entries.
  - lib 1,486/1,486; `pipe_mode` 13/13.
- Frozen items are unchanged.

## Perturbations

- **Sprite directory:** 119 pairs (AT 35, AS 41, AQ 43) plus the 3 AS include resources, in `test-diagrams/perturbations/sprite-command-post-tim-ownership/`. CLI result: 76 pass and 43 fail.
- **Participant directory:** 23 pairs in `sequence-participant-silent-residue/`.
- **Library harness after packaging:** 70 and 16 failures in these two new directories only, and 0 new elsewhere (`gates/post-package.log.gz`). The library harness also counts preserved Java error pages as failures; the CLI result above does not.
- **Older pairs:** the 790 and 28 pairs stay preserved under their original mechanism directories.

## Next obligations

See `review.json` → `next_obligations`.
1. Own every body that Java's earlier-registered multiline commands own before the sprite dispatch.
2. Never strip comments inside an open sprite command.
3. Emit `!startsub` bodies in place.
4. Rerun AT, AS, AQ, 790 and 28 with zero losses.

`support/` holds every script; `runs/` holds per-row exits, hashes, stderr and comparator output.
