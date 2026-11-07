# Checker re-review 2: Menlo Bold Italic fallback (🎯T14.50)

Verdict: **accepted** for this mechanism. The packet that carries it is not
yet accept-ready, because `menlo-font-name/review-2.json` is rejected on a
decimal-ratchet regression in `text_render.rs`. See `../review-2.json` and
`scoreboard.json`.

## Identities

Identities are as in the font-name review:

- tip `358b6e285e6511fafb28af4f4527b4e3b1e2bf80`, whose code is `0063e0e77`;
- merge base `0cd7743aa8fe66a01e0be1e5b1c2a66aae34feaa`, which already
  contains the fallback commits;
- pre-packet `505999cac2e0ddd6913046d6c6a9bc15eb8dea45`;
- Java `71806a23780b04a5ccde2f8ceb5121edad5eb711`.

## Provenance check

All 267 `FALLBACK` rows in `crates/rustuml-render/src/menlo_bi_fallback.rs`
equal the 267 `JRS BOLDITALIC FALLBACK` rows of the relay probe
`grok-g3-menlo-jrs-probe2/probe.stdout` (SHA256 `31bd13b0…`). The scalars,
PostScript faces, unitsPerEm and integer advance units all match, with 0
mismatches. Apple Color Emoji is the only non-integer case: 7600/7 of an
800-unit em, which is 19 pt observed at 14 pt.

## Scores

Scores are pre / base / tip, with no losses anywhere:

| Batch | Pre | Base | Tip | Total |
|---|---|---|---|---|
| CZ | 24 | 47 | 51 | 68 |
| CQ | 0 | 7 | 7 | 7 |
| CR | 0 | 13 | 13 | 14 |
| c9a | 100 | 108 | 112 | 288 |

The CLI corpus has 0 golden stdout changes and 0 perturbation losses.

## Pairs

The pairs are under `test-diagrams/perturbations/menlo-bold-italic-fallback/`:
7 CZ cases with their relayed Java SVGs, all passing. They cover the Monaco
and Hiragino fallbacks, the Menlo Bold control, and `asFontFace` for
`bolditalic`, `BoldItalic` and `700`.

## Residue

The files in `residue/` are CZ sources plus their Java SVGs. They are kept out
of the executable walk because an accepted review may not list failing pairs.
Each tip value equals or improves on pre.

- `opus2_fb_menlo_bi_emoji13`: Java 17 at 13 pt; Rust 17.6429. With CR
  `g3_bi2_emoji_12` (Java 16, Rust 16.2857), this shows Apple Color Emoji
  advances are integers per size. It widens 🎯T14.50.2.
- `opus2_fb_menlo_bi_astral`: U+1F4A1 is 19 in Java and one cell in Rust. The
  account's invariant covers it, but the 267-scalar table does not.
- `opus2_fb_menlo_bi_combining`: `e` + U+0301 is one cell in Java and two in
  Rust. The combining mark has zero advance in Java's layout path.
- `opus2_fb_menlo_italic_jcaron`: Menlo Italic falls back too. U+01F0 uses
  Monaco 8.4014 and U+2602 is 14, against one cell in Rust.
- `opus2_fb_courier_bi_cloud`: Courier New Bold Italic U+2602 is 14 in Java
  and one Courier New cell in Rust.

## Proposed targets (not filed)

- Menlo Italic, and Courier New Bold Italic, should measure JRS fallback
  advances for the scalars their own cmap lacks.
- Emoji fallback advances should be the per-size integer Core Text returns,
  not a linear scale. This extends 🎯T14.50.2.
- Astral scalars that Menlo lacks in every face should measure the JRS
  fallback face.
- A combining mark after a base character should have zero advance in Menlo
  widths.
