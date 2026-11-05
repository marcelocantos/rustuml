# Sequence rejection repair: maker packet

Implementation: `8291416539aa9c61589e4dd586ad65d3a135966c`. Prior checker rejections remain in force; this is not an acceptance record.

All 144 valid checker inputs, 22 retained failures, 535 prior maker inputs and 56 valid new inputs match through the real CLI with the unchanged comparator. The 70 new inputs include 14 invalid negative cases, all retained. The original 41 grouped-note probes remain 39 matches and two unresolved font controls.

The frozen 955 cohort is unchanged at 403 matches, 542 valid mismatches and 10 invalid references. Across all 1,728 entries: 1,160 matches, 542 mismatches, 26 invalid references. There are 1,680 distinct source byte strings and 48 duplicate entries. Entries are never deduplicated for the reported cohort counts.

On 1,658 entries with preserved independent-checker before evidence, CLI matches increase from 970 to 1,104: 134 entry recoveries and zero lost passes. The checker candidate production tree is byte-identical to starting revision `1800ac48714743863307c82674490c697fcf5709` (including Cargo files). The 70 new cases have no recoverable before outputs after scratch loss; no before parity claim is made for them. `before-after.json.gz` names every comparison and its original evidence source.

No real CLI panic, registry cascade, or parse error on a valid reference was observed. Six orphan-return negatives now exit 1. Twenty other invalid references still produce SVG, including ten newly retained sequence negative probes. `negative-input-status.json` preserves all statuses. The `diff_one` adapter still panics on the pre-existing empty invalid-date Java reference; this is not a renderer panic.

Both release main tiers were rerun only because the original scratch logs and binaries disappeared: product library 11,181/11,251 and assisted library 10,497/11,251, unchanged; sequence 1,411/1,411. These are library tiers, not 11,251 CLI processes. Parser/render units: 565/565 and 782/782. The builtin 1,123 heldout suite has 568 matches and 555 failures (12 invalid references, including one empty reference). Guards remain 4/6. Full logs are under `logs/`.

The frozen include source depends on a missing support file. Commit `a8096d8c7073e1ba09d41e07dcef303b8d33eee1` restores it byte-for-byte from parent `464de61965761a1bc778e8bafca798ba8baefe55`. The exact same binary passes this case at repository-root cwd and fails at Cargo-package cwd (T14.6.2). The first replay without support and both follow-up captures remain in `cli-captures.tar.gz`; `measurements.json.gz` explicitly selects the complete root-context case. No full corpus rerun was needed for that evidence-only correction.

Formatting passes for owned files; workspace formatting remains red in pre-existing class files. Strict Clippy is blocked first by parser/component.rs dead code; renderer-only Clippy exposes 22 retained findings (15 sequence findings in unchanged lines, seven in other renderer files). Two new needless borrows were corrected before the recorded build. No baseline, comparator, exclusion, target or checker record was changed.

## Causal implementation

Return color is parsed as arrow configuration and orphan destinations are rejected. Reverse self-arrows share preferred dimensions, before-constraints, live endpoints and paint. Notes use component preferred dimensions for scheduling, spans, group bounds and paint. Page title/legend/caption/footer composition uses body dimensions and wrapper order. Teoz uses raw lifecycle history, mode-specific future flags, ordered tile callbacks retained through measurement/background stages, and an independent staircase painter. Captured Real note bounds remain distinct from final paint allocation. Delay whitespace and box titles retain allocation-bearing source content; Teoz englober margins feed participant constraints and frame anchors.

| Mechanism | Original causal-account commit | Implementation commit(s) |
|---|---|---|
| sequence-box-title-grammar | `55e88b2add0a1356aeb1bc42284f1d5a0f06b974` | `8291416539aa9c61589e4dd586ad65d3a135966c` |
| sequence-delay-label-allocation | `a21a02753cc9abaeef64d7b75caacb253d4f4c7c` | `8291416539aa9c61589e4dd586ad65d3a135966c` |
| sequence-page-wrapper-hierarchy | `836f9c0d6e9337c7af51d87ee250b8698b23fd7c` | `cd6be3da975cc9a8133e91bb33aceb28c7f6da3f`, `8291416539aa9c61589e4dd586ad65d3a135966c` |
| sequence-preferred-component-flow | `11c4e6354cd2029fad6463db0c14a023e422a6d6` | `2000a77ec57266181a4ae73ee73e65381ed1bf36`, `8291416539aa9c61589e4dd586ad65d3a135966c` |
| sequence-return-configuration | `f785ee820f7364f0669a620222ac1d6e64c42d83` | `e120da16f6ae07ce55edb7dedd9db80f800b4f2a` |
| sequence-reverse-self-component | `a519af2de609190c9c124c5b5aa02a8dc06b9f97` | `2dd42f4c71422afcbb2246f5ed3137cdf70c747f`, `8291416539aa9c61589e4dd586ad65d3a135966c` |
| sequence-teoz-callback-lifecycle | `7ddde17123b246df7ac2e4084e27e21f2deb450a` | `8291416539aa9c61589e4dd586ad65d3a135966c` |
| sequence-teoz-englober-margins | `3c9a9e67c7742ef7a70a4645ba499073366c069f` | `8291416539aa9c61589e4dd586ad65d3a135966c` |
| sequence-teoz-event-history | `836f9c0d6e9337c7af51d87ee250b8698b23fd7c` | `8291416539aa9c61589e4dd586ad65d3a135966c` |
| sequence-teoz-group-tile-composition | `bf4f0c1410a2dc8d72e76d9b470431fe58c5f6c6` | `8291416539aa9c61589e4dd586ad65d3a135966c` |
| sequence-teoz-note-component-flow | `81acbf505408497b108442bab5371fa9e78f9a5e` | `8291416539aa9c61589e4dd586ad65d3a135966c` |
| sequence-teoz-real-bound-capture | `a5a826c28d39e4a1ac01348390c3c4f3bfa2cc48` | `8291416539aa9c61589e4dd586ad65d3a135966c` |

New account schemas were losslessly normalized with explicit parent authorization in `89cac979`. `account-normalization/manifest.json` records the original commit, original Git blob, SHA256, exact archived bytes and field-copy mapping. Original causal fields remain intact. The normalization date is separate from the preimplementation account date. No checker verdict was authored.

## Reference and execution provenance

No local Java or javac was launched during recovery. Sixty new SVG references and both source probes were captured earlier on Monkey Man through the pinned headless wrapper in private directory `/Users/marcelo/rustuml-reference/scratch/worker.gKRm76Fl`. Ten return references were already captured before the local-Java pause and were reused. Every source/reference and stderr is preserved in its mechanism maker-results directory; per-case hashes are indexed in `new-reference-inventory.json`. The remote runtime is Homebrew OpenJDK 21.0.5 and Graphviz 12.2.0; origin runtime differs. The JAR SHA256 is `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`, source revision `71806a23780b04a5ccde2f8ceb5121edad5eb711`. Timezone is GMT+11:00. Seven calibration pairs establish scoped agreement, not global runtime equivalence. Exact parent contract snapshots are included.

Confirmed actual remote invocation: `directional_0.puml` produced 4,741 SVG bytes with exit 0 and empty stderr. The saved remote helper uses the authorized wrapper; source probes and their stdout/stderr are preserved under `sequence-teoz-event-history` and `sequence-teoz-callback-lifecycle`. Java environment audit found no unguarded saved invocation; all audited prior local generators had explicit headless flags. Local launches stayed paused, then moved entirely to Monkey Man.

The diagnostic `HistoryPaintProbe` SVG remains on the remote scratch path; retrieval during recovery failed with SSH exit 255 (connection timeout). Its source and stdout/stderr are preserved, and `probe-svg-retrieval.json` records the failed retrieval. This missing diagnostic artifact is separate from the 70 complete input/reference pairs and was not substituted into any comparison.

Current Rust build and captures are in `/Users/marcelo/Library/Caches/rustuml-sequence-repair`. Cargo jobs were capped at 4; the target is private. `binaries.json` identifies exact binaries, comparator compilation and source hashes. `checks.json` names commands and exit statuses. CLI measurements used immutable checker `measure.py` and `compare_cli.rs`; the source is passed unchanged through `rustuml -tsvg -pipe`. The comparator is unchanged. `package_evidence.py` performs no Java launches and packages existing evidence only.

## Remaining dependencies

Both logical-font controls remain failed under parent target T14.11. SansSerif has a family-attribute quoting discrepancy; Serif also has different metrics. Existing physical-font and shared hyperlink serialization failures remain. No host fonts were imported and no geometry was tuned around them. The ten invalid sequence negatives remain semantic dependencies: reactivation after a close on one self-message and invalid spaced right-border syntax. The full 542 valid failures and 26 invalid references are in `remaining-failures.json.gz`, with dispositions in `dependencies.json`. They are not waived.

Parent owns the larger 1,449 cohort and combined integration. This branch preserves its frozen 1,123 heldouts and adds maker evidence outside the standing fixture directory. New independent review remains required before acceptance.
