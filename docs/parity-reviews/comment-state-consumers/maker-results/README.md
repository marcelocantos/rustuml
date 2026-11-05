# Shared-SVG comment/state corrective round: maker results

Maker: Claude Opus 5.5 (Claude Code in T3 Code), branch
`maker/shared-svg-corrective-claude`. Started from rejection `7ed71f3e5` and
merged `parity-recovery-2026-11-03` (`9834a7194`) first. This file records the
maker's measurements only. It is not acceptance; an independent checker must
judge the result.

Java pin `71806a23780b04a5ccde2f8ceb5121edad5eb711`. Every account below was
committed before its draft.

## Commits

| Commit | Role |
|---|---|
| `5e8dbd688` | Merge of `9834a7194`. Conflict in the `filter_registry.rs` tests resolved by keeping both sides; the base `RegistryScope` unwind guard is unchanged. |
| `258995f33` | Account: delay spacer-run measurement (`sequence-delay-label-allocation/maker-results/spacer-fragment-account.json`) |
| `6830bd056` | Draft: measure the delay label's leading spacers as an atom run |
| `e2431c84e` | Account: state cluster and history comment selection, plus `tools/comment_lists.py` |
| `2bb3b829c` | Draft: cluster comments only for non-autarkic groups; history endpoints keep their parent scope and names |
| `00a5f9ae9` | rustfmt of the two hunks above (no behavior change) |
| `e349cb2d7` | Account: deployment Quark identity and link reversal |
| `db75de354` | Draft: `DeploymentNode.quark_name` and `LinkType`-derived reversal |
| `7704a34c9` | Account: StateSkin stroke-scope boundary |
| `b27411107` | Draft: StateSkin line roles through `asColor` and `fixColor` |

## Obligations

1. **T14.24: not fixed, and blocked on an owner decision.** Pinned Java
   `Pipe.readSingleDiagram`, `BlockUmlBuilder` and `UmlSource` keep leading
   blank lines. Only picoweb GET's `DiagramSourceCompressor` trims them, and the
   three goldens came from GET. The owner contract `b62d8ce96` (already in the
   base) selects the raw behavior and names these three goldens as documented
   historical mismatches. The branch therefore matches pinned Java pipe output,
   and T14.24's acceptance contradicts that contract. Details are in
   `../../tim-evaluated-event-stream/maker-results/leading-blank-source-identity.json`.
   The `preproc_complex_theme_override` gain is genuine: it comes from the
   evaluated theme stream.
2. **Delay merge interaction: fixed.** The causal pair is the shared-SVG port
   of `StripeSimple.getAtoms` (an empty stripe gains one space atom) and the
   sequence packet's leading-spacer width, which measures a prefix of the
   stripe through the whole-line API. With no leading spacer, that adds one
   phantom space: +3.48046875 at 11 pt. The fix is a stripe-prefix (atom-run)
   measurement.
3. (a) **Comment identity and cluster selection: fixed for 1162, 1164,
   `state_history_nested_identity`, `deploy_alias_collision` and
   `group_identity_collision`.**
   (b) **Stroke scope: made explicit, with the boundary documented in the
   account.**
   (c) **Missing producers: residual.** Activity alias and partition admission
   are parser and layout gaps, not bounded.

## Measurements: before `9834a7194`, after `b27411107`

Both trees were private release builds with `-j 6`.

- **No-oracle:** 11181/11251 before, 11179/11251 after.
  - Lost: `activity/act_skinparam_shadowing`,
    `activity/act_skinparam2_shadowing_true` and
    `edge-cases/edge_whitespace_multiple_blank_lines`. These are the
    GET-front-end goldens covered by obligation 1.
  - Gained: `preprocessing/preproc_complex_theme_override`.
  - `edge_seq_delay_texts` and `edge_sequence_delay` are no longer lost.
  - Failure-name files are in `measurements/`.
- **golden_pairs (assisted):** 10497 passed, 754 failed and 1299 skipped, both
  before and after. The failure-name files are identical.
- **parity_perturbations:**
  - 3008 base sources: 1643 pass before, 1643 + 105 = 1748 pass after, with 0
    new failures.
  - The after corpus has 3798 sources: 2196 pass and 1602 fail.
  - 790 sources were added by the packet's ancestry; 342 of them fail. Nine of
    those are new sources in pre-existing directories, all inherited TIM
    rejection counterexamples from `7bb210e9f`. None of them is a new failure
    of a previously existing source.
- **harness_guards:** 4/6 both before and after. `decimal_literal_ratchet` and
  `parity_review_records_are_complete` fail in both.
  - The decimal ratchet gains `svg.rs: 0 -> 1`, from the packet; these
    commits add no decimal literals.
  - Review-record categories grow only through the packet's rejected records:

    | Category | Before | After |
    |---|---:|---:|
    | latest accepted review cannot contain failing perturbations | 1012 | 1300 |
    | perturbation is not named in any checker review | 198 | 306 |
    | latest checker review must be accepted | 151 | 198 |
    | No such file | 13 | 15 |

    The two new `No such file` entries are `shared-svg-corrective/account.json`
    and `tim-corrective-execution-maker/account.json`, both from the packet.
  - These commits add no `review*.json` and no perturbation files.
- **pipe_mode:** 17/17, with strict XML green. `pipe_mode.rs` is unchanged
  since `7ed71f3e5`, so the expected-malformed list is unchanged.
- **Workspace lib:**
  - Before: 586 parser and 800 renderer tests, all passing.
  - After: 649 parser tests pass, and 825 of 827 renderer tests pass.
  - The two failing tests are inherited from the packet:
    `filter_registry::tests::parser_source_identity_matches_java_shadow_id_after_initial_blanks`
    (obligation 1) and
    `class::tests::plain_theme_uses_legacy_class_icon_and_font_defaults`.
- **Comment-list oracle** (`tools/comment_lists.py`, 16348 pairs):
  - Exact lists: 14270 with the base binary, 14943 after the merge and delay
    fix, 14952 final.
  - Final against the merged tree: 9 gained, 0 lost.
  - Final against the base: 685 gained, 3 lost. The three losses are
    `unicode_activity` and `surrogate_display_residue`, where the Java
    reference is a crash SVG (T14.20), and `sub_suffix_continuation`, where it
    is a Java syntax-error page. All three are nonpositive references.
- **Whole-output byte changes per draft** (`tools/output_hashes.py`):
  - Cluster/history draft: exactly the 6 intended perturbations.
  - Deployment draft: exactly 4.
  - Stroke draft: exactly 2.
  - In each draft's diff, `edge_seq_title_with_variables` (`%date()`) also
    appears; it is clock-dependent, not a code change.
- **Lost passes, rejections, errors and panics.** CLI exit statuses and panics
  for all 16348 golden and perturbation sources were compared between the base
  binary and the final binary.

  | Event | Total vs base | From these drafts |
  |---|---:|---:|
  | Lost passes on pre-existing sources | 0 perturbation, 3 no-oracle (obligation 1) | 0 |
  | New nonzero exits where the Java reference is an error page (rejections gained) | 69 | 0 |
  | New nonzero exits where the Java reference is a valid diagram (new valid-input errors) | 10 | 0 |
  | Base panics that now exit 0 with a valid Java reference | 3 | 0 |
  | Lost rejections | 1 | 0 |
  | New panics | 0 | 0 |

  - Panics fall from 8 to 4.
  - Every one of these status changes is already present at the merged tree
    (`6830bd056`), before any comment or stroke draft, so they come from the
    packet's TIM ancestry. Each draft's whole-output diff contains only its
    intended files.
  - The ten new valid-input errors are six under `tim-external-sub-selection/checker3_*`,
    `tim-function-execution-kind/checker3_include_from_return`, two under
    `tim-intval-conversion/checker6/intval_unicode_*`, and
    `tim-parser-error-propagation/checker2_sub_ignored_prelude`.
  - The lost rejection is `surrogate_display_residue`. Its Java reference is a
    crash SVG; the base panicked, and the branch now renders a normal diagram
    (T14.20).

## Residuals

- **T14.24 contract conflict.** Owner sign-off is needed to change the target,
  or to change the renderer unit test as was done for the parser assertion in
  `136ee057e`.
- **Autarkic state groups.** Rust still paints groups that Java treats as
  autarkic through `emit_root_state_cluster`: a `<g class="cluster">` wrapper
  and cluster geometry. Only the comment producer now follows Java. The
  consumer dispatch and geometry stay open.
- **Quark identity outside deployment.** Class and usecase still infer
  identity (T14.23). Nested custom namespaces also remain open.
- **Strokes outside the StateSkin boundary.** Stereotype, per-state `##color`
  and per-arrow `-[#color]->` strokes, the document background casing, and the
  `circle_root_alpha` width (89 vs 88) remain open.
- **Missing producers.** Activity aliases and partitions remain open, and the
  T14.20 crash cases keep Rust comments in normal diagrams.
- **Fresh case `delay_legacy_short`.** Rust routes this sequence input to
  CLASS. This dispatch bug predates these commits.

## Fresh reference relay (pending)

Twenty held-out cases outside the golden corpus:

- 0, 1 and 3 leading blank lines, each in activity and class with shadowing
  on;
- delay labels of different widths, legacy and teoz, with and without leading
  spacers;
- nested history and cluster selection;
- deployment alias and reversal;
- stroke roles.

They are staged at `~/Library/Caches/rustuml-shared-svg-claude-maker/refs-batch1/`:

- `reference-manifest.json`, sha256
  `6eea649b63312e20a90d0aebce55bb6f61d3f167a6b0c49263f187cf4c29017b`;
- `reference-input-bundle.json.gz`, sha256
  `2ed23e538b8b7ae3983ad0bef501ea28b92015d821d13f39db47c6320ba43cad`.

Rust before/after outputs for them are cached beside the bundle. They will be
compared, and registered under `test-diagrams/perturbations/comment-state-consumers/`,
once the references arrive.
