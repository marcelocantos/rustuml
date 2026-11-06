# TIM line continuation (🎯T14.22) round 4: independent checker (pre-registration)

Checker: Claude Opus 5.5 (`claude-opus-5-5`), branch `review/tim-continuation-round4-claude`, reviewing maker tip `fa3f95010` (code `9ed52f435`, merged base `1f6ffa9f4`).

This commit only pre-registers held-out batch **CR4** (74 cases, 21 resources). No Java result for these inputs exists yet. Java was never run locally.

| File | sha256 |
|---|---|
| `pending-request-cr4/reference-manifest.json` | `55029e322501c927c024ea1d8275c6abf17074a173c4d175b9f1a82056ed5dec` |
| `pending-request-cr4/reference-input-bundle.json.gz` | `bd1c8bae09d7de4d54a024407754f601a58bf78a1fc56365d5ad6cda316d0d67` |
| `pending-request-cr4/cr4-rust-predictions.json` | `805a91cab523b550ce53eee906065e0d84461c34f209403d4a90db280e0d0601` |
| `pending-request-cr4/cases-with-java-predictions.json` | `adad05098fba5e853d17c03eff2f7fe3389515d0b00c9ec75e5b6f2faaeefef2` |

The immutable copy is at `~/Library/Caches/rustuml-tim-continuation-round4-claude-checker/relay-cr4/` (read-only). The manifest's source paths point at its `inputs/`. Rust outcomes are recorded for `9ed52f435`, `1f6ffa9f4`, `93bf53e18` and `9834a7194` (fresh `git archive` release builds; `binaries.sha256`). Java predictions marked `unsure` in the axes are recorded as guesses, not traces.
