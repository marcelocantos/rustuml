# Checker re-review 2: Menlo font name (🎯T14.45)

Verdict: **rejected**, on one process defect. The mechanism passes every
held-out case. See `../review-2.json` for the full findings and
`scoreboard.json` for the numbers.

## Identities

- Packet tip `358b6e285e6511fafb28af4f4527b4e3b1e2bf80`. Its code is
  `0063e0e77a66e457bf37aa21e93d19b1bc480283`; the last commit changes only
  `bullseye.yaml` and docs.
- Merge base `0cd7743aa8fe66a01e0be1e5b1c2a66aae34feaa`. That commit merges
  parity-recovery into the maker branch, so it already contains the 🎯T14.50
  fallback commits. The parity-recovery parent
  `505999cac2e0ddd6913046d6c6a9bc15eb8dea45` ("pre") was also built so that
  the whole packet could be attributed.
- Java `71806a23780b04a5ccde2f8ceb5121edad5eb711`, relayed results only. No
  Java was run locally.

## Blocking finding

`221a17da0` adds `let mut menlo = base(26.0, 76.6016);` to a
`text_render.rs` test. The line has no provenance comment and the baseline was
not updated. `decimal_literal_ratchet` therefore lists
`text_render.rs: 51 -> 52` at the tip, and it does not list that file at the
merge base. Counted with the guard's own rule, the file has 51 literals at
`cb550c0d6`, `505999cac` and `0cd7743aa`, and 52 at `358b6e285`. The value
repeats the `base()` argument on line 2025, so reusing that binding fixes it.
A re-check needs `harness_guards`, the workspace lib tests, and a
byte-identical CZ stdout check.

## Held-out batch CZ (unseen by the maker)

- `rust-predictions.json` (`debbce39…`) was registered at 10:25:29, before
  the results were relayed at 10:28:17. My own rebuilds reproduce it
  byte-for-byte: 136/136 (exit, stdout and stderr hashes).
- Scores (pre / base / tip): 24 / 47 / 51 of 68. There are 0 losses, whole-SVG
  or element-level.
- For this mechanism, the tip misses are:
  - `g3r2_creole_decode_edges`: one element remains. The `SansSerif` family
    spelling is 🎯T14.98.
  - `g3r2_sk_empty`: Java writes `font-family="''"`.
  - `g3r2_sta_ignore_stereotypefont`
  - `g3r2_fs_seq_arrow_*`: `sequence.rs` reads `arrowFontStyle` with
    `contains`.

  Every one of these is identical in pre, base and tip.

## Pairs

The pairs live under `test-diagrams/perturbations/menlo-font-name/`:

- `opus2_*`: 13 CZ cases with their relayed Java SVGs. 11 pass and 2 fail
  (`opus2_creole_decode_edges` and `opus2_sk_empty`).
- `chk_*`: the six pairs from the earlier checker. 5 pass and 1 fails
  (`chk_skin_helvetica_light`, unchanged since pre). Their review,
  `review.json` from `9c88c612b`, is carried onto this branch byte-for-byte
  so that every pair is named in a review.

## Proposed targets (not filed)

- Sequence `sequenceNoteFontName` should be ignored. `SkinParam.cleanForKeySlow`
  rewrites only `sequence(participant|actor)`, and `FromSkinparamToStyle` has
  no `sequenceNote` key. This comes from reading the Java source and needs a
  relayed probe.
- Every `FontStyle` reader should follow `ValueImpl.asFontFace`. The readers
  that still use `contains("bold")` are the sequence arrow
  (`sequence.rs:7099`), the class note (`class.rs:13377`), state
  (`state.rs:2558`, `state.rs:9350`) and the activity title
  (`activity.rs:994`). CZ `g3r2_fs_seq_arrow_bolditalic` and
  `g3r2_fs_seq_arrow_css700` are witnesses.
- The SVG `font-weight` should carry the CSS weight. Java writes 800 for
  `bolder` and 300 for `lighter`; Rust writes 700 and nothing
  (`g3r2_fs_class_bolder`, `g3r2_fs_class_lighter`).
- An empty `defaultFontName ""` should be written as `font-family="''"`
  (`g3r2_sk_empty`).
- The SVG NBSP rule should apply to the logical `Monospaced` family. The
  `c9a_fs_mo_*` and `c11a_of_*_mo_*` perturbations are witnesses.
