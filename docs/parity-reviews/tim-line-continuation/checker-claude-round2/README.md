# TIM line continuation (🎯T14.22) round 2: independent checker, pre-registration

Checker: Claude Opus 5.5 (`claude-opus-5-5`), branch `review/tim-continuation-round2-claude`, reviewing maker tip `e3aa1c6b4` (code `f2d12d0ef`). **This commit is a pre-registration only; no verdict yet.**

The held-out batch **CR2** (53 cases, 41 predicted Java-valid, 10 include resources) was staged and the Rust outcomes at all four builds were recorded before any Java result for these inputs existed. Java was not run locally.

| File | sha256 |
|---|---|
| `~/Library/Caches/rustuml-tim-continuation-round2-claude-checker/cr2/reference-manifest.json` | `eadfd605b47bf8f1eb556f117654d983a9752b90264fa74864c643923df97b77` |
| `~/Library/Caches/rustuml-tim-continuation-round2-claude-checker/cr2/reference-input-bundle.json.gz` | `d7717be2ff88503dc15151586625a263c123a86735fe011232983fd2e3f35e84` |
| `pending-request-cr2/cr2-rust-predictions.json` (Rust exit, stdout hash, texts, source lines at `f2d12d0ef`, `b0fb43c3e`, `93bf53e18`, `9834a7194`) | `8ffb84f505f4e4d08e9d81227f2bf90f27e283b7f9610d416e86eb0fb1da3b02` |
| `pending-request-cr2/cases-with-java-predictions.json` (checker's Java predictions from the source trace) | `e0a83502c6963474d218516e1446a5c5fef809ad53caefadb198bf9f04d890f2` |

Builds: `git archive` of each tree, each with its own `CARGO_TARGET_DIR` (`support/build.sh`). Binary hashes are in the predictions file.
