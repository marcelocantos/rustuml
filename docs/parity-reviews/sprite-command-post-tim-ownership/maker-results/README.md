# Post-TIM sprite command ownership: maker results (corrective round)

Maker: Claude Opus 5.5 (`claude-opus-5-5`), branch `maker/sprite-participant-corrective-claude`. This is a maker record, not a verdict; an independent checker decides. No push, PR or release.

Starting point: rejection `dc1dfbb4a` (maker production `d15d6eaf9`, `ba2e6c2b7`) merged with `parity-recovery-2026-11-03` (`9834a7194`) as `93bf53e18`, the **before** tree. The merge had no conflicts and brought no new crate code. Goldens at `9efdc12f`. Java `71806a23`, JAR `aa5262a2…`. Hashes are in `bindings.json`.

| Commit | Role |
|---|---|
| `ff761cd0a` | account (`account.json` here), before any draft |
| `95cf85252` | production: post-TIM sprite command group |
| `65b5a1633` | account for participant residue (sibling mechanism) |
| `ecbd75ec8` | production: participant tooltip and residue warnings |
| `87bdb0feb` | production: created head drawn without link anchor |

`../account.json` is the guard-schema copy of this account.

## Mechanism (Java trace)

`BlockUml#<init>` runs `TimLoader#load` (via `TContext#executeLines`, which resolves `!if`, `!procedure` and `!include`) before any command sees a line. Sprite declarations are diagram commands: `CommonCommands#addCommonCommands2` registers them in this order: `CommandFactorySprite` multiline, single line, `CommandSpriteSvg`, `CommandSpriteStdlib`, `CommandSpriteStdlibSvg`, `CommandSpriteSvgMultiline`, `CommandSpriteFile`. `PSystemCommandFactory#getCandidate` offers each command-start line to the factory's commands in order. A multiline command that never completes (`CommandMultilines2#isValid` stays OK_PARTIAL to EOF, so `isMultilineCommandOk` returns null) falls through to the next command. Multiline note, legend, ref and class-body commands own their bodies, so body text is never offered.

Changes:
- `preprocess/mod.rs`: all sprite state, the body branch and the declaration block are removed from `process_one_line`. In the sprite area only, a line in the `CommandSpriteSvg`/`CommandSpriteSvgMultiline` start grammar is exempt from the Rust-only inline `'` comment stripping. Java has no such stripping: `text.StringLocated#removeInnerComment` removes only `/' … '/`. No other TIM code changed.
- `parse/sprite.rs` (new): the seven grammars, which a unit test keeps identical to `REGISTERED_SEQUENCE_COMMANDS` 22–28, plus CommandMultilines2 completion and the execution failures. `CommandFactorySprite#executeNow`/`#executeInternal` fail with "No sprite defined." and "Only 4, 8 or 16 graylevel are allowed."; `CommandSpriteFile#executeArg` fails with "Cannot read: <src>". An unclosed pixel header therefore fails as in Java, through `CommandSpriteFile` with FILE=`[2x2/16] {`. An unclosed SVG header has no candidate and is a syntax error.
- Family positions, from each factory's `initCommandsList`:
  - Sequence: the existing registry, which already has Java order and completion, plus execution failures.
  - `ClassDiagramFactory`, used for class and object diagrams: `addCommonCommands2` comes last, so a line is offered only after every class command declines it.
  - `StateDiagramFactory`: `addCommonCommands1` comes after the state, link and note commands, so the line is offered last. The structural pre-pass skips the same spans.
  - `DescriptionDiagramFactory` (usecase, component, deployment), `ActivityDiagramFactory3` and `ActivityDiagramFactory`: `addCommonCommands1` comes first, so a command-start line is offered right after the open-body handlers.
- `parse/mod.rs`: for families whose dispatch position is not ported (timing, archimate, mindmap, gantt, wbs, salt, nwdiag, regex, board, ebnf), complete sprite commands are consumed context-free after TIM. This is the former TIM behaviour moved after TIM; it is recorded as residual. Subtype detection sees complete sprite commands masked.

## Measurements (actual CLI, unchanged `compare_cli`, four modes; before = 93bf53e18, after = 87bdb0feb)

| Cohort | n | before match/mismatch/valid-error/rejected/accepted-invalid | after |
|---|---:|---|---|
| retained (checker-claude manifest) | 4,174 | 2645/1076/17/368/68 | 2658/1063/17/370/66 |
| goldens | 12,550 | 11181/70/0/201/998 (+100 compare-101) | 11181/70/0/202/997 (+100) |
| fresh main (790) | 790 | 176/331/6/261/16 | 191/322/0/273/4 |
| fresh supplement | 28 | 0/14/8/2/4 | 8/14/0/6/0 |
| fresh batch1 (relay, 43) | 43 | 4/22/8/4/5 | 31/3/0/8/1 |

Counted separately:
- **Lost matches:** 0 in every cohort, both against `93bf53e18` and against the pre-packet baseline `c525fd32a` (the checker's captures).
- **Lost rejections:** 0.
- **New valid-input errors:** 0. Retained valid errors stay 17: the approved 15 (`new-valid-errors.json`, SHA256 `c3076b2c…`, exactly 15 sources) plus the two pre-existing short external CREATE entries.
- **Panics:** unchanged. `--yaml` panics on 63 retained and 148 golden sources in both builds (`serde_yml`). There are no SVG panics or timeouts.
- **Golden SVG bytes:** 1 of 12,550 changed. `edge-cases/edge_render_sprite` has a Java error golden and is now rejected.

All 10 valid-input errors in the rejection are fixed: `c75s_inactive_unclosed_*`, `c75s_macro_uninvoked_unclosed_*` and `c75_include_pixel_into_include_*` now match (legacy and teoz). So do the 4 include cases that were still errors (`c75_include_parent_to_child_*`, `c75_include_sibling_*`). Of batch1, 27 now match, including every note/legend/ref/class-body `Sprite…` case, the class links and the inline SVGs with ` '`. Four Java-invalid batch1 cases are newly rejected.

Intermediate candidates on the 861 fresh cases are in `intermediate-candidates.json`.

Model changes (`runs/*-model-changes*.json`):
- `meta.source` (post-TIM UmlSource) now contains the sprite lines instead of blanks, as Java's does: 51 golden, 33 retained.
- The additive AST field `url_tooltip: None` (sibling mechanism).
- HashMap ordering.
- In two fresh cases, rows from `c75_sprite_body_if_*` now come from TIM-resolved lines.
- `c75_sprite_procedure_*`: `data-source-line` after a content-emitting procedure moves from 11 to 12; Java has 10. This is the general TIM procedure rule, previously masked by the sprite blanking. The case is a mismatch both before and after.

Gates (`gates/`):
- no-oracle 11,181/11,251. Failure names are byte-identical to `9834a7194` (70). The ratchet message (class/preprocessing/rendering not locked) is the same pre-existing one as on the base, and the baseline is untouched.
- golden_pairs 10,497, with failure names identical to the base (754).
- parity_perturbations: 2,366 failures after versus 2,396 before. There are 0 new failures and 30 are fixed.
- harness_guards: the same 2 failing guards as the base. The decimal ratchet is unchanged. The review-record guard gains only `account has no independent checker review` for the two new mechanism directories, which is the pre-review state.
- pipe_mode 13/13; workspace lib 1,486/1,486 (before 1,477).
- `rejection_contract.py` on 292 Java-invalid fresh inputs: 267 rejected before, 287 after, 0 losses. It exits 1 on 5 remaining false accepts, listed under Residue.

## Residue (explicit)

- Pre-existing false accepts:
  - `c75_sprite_compressed_*`: the `SpriteGrayLevel#buildSpriteZ` decode is not ported; the line is reported as unmaterialized.
  - `c75_sprite_function_*`: a `!function` without `!return` is a TIM issue, not a sprite one.
  - `cl_class_bare_sprite`: the Rust class parser silently ignores unknown lines.
- Sprite rendering (`c75_use_*`): the line warns as recognized/unmaterialized. Pixel sprites are drawn only by the class and deployment renderers.
- Families on the context-free fallback: timing, archimate, mindmap and the other non-UML families.
- `CommandSpriteFile` resolves FILE against the process cwd (Java: `FileSystem` current dir).
- Description/activity handler leniency inside bodies that Rust does not model as body states.

## Pending relay request (batch2)

`pending-request/` holds 54 held-out cases (33 intended valid) and 3 include resources. They cover `!if` false/true and `!else`, uninvoked and invoked procedures, `!include` across boundaries, `Sprite…` text in notes, legends and class members for each family, unclosed blocks per family including the fallback families, execution failures, colour-free geometry controls and participant probes. `reference-manifest.json` SHA256 `262c9c1e…ebc06f`; `reference-input-bundle.json.gz` `c337a3d9…c8ec8ac`; see `SHA256SUMS`. The immutable private cache is `~/Library/Caches/rustuml-sprite-claude-maker/batch2/`. `local-observations.json` holds local CLI exits only, with no Java verdict.

`support/` holds every script; `replay.py` is the checker's copy with only `max_workers` 4 → 8.
