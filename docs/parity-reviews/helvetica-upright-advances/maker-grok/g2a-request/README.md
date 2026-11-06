# Held-out Helvetica probes (G2A), registered before any Java run

Code commit of both binaries' source: base `cb550c0d67cc729ce764895005b7e86310c9f313`, tip `079fa29890b3e6230bfa5c78f979375dad5500c3`.
The docs commit that records this file is later and does not change the renderer.

Immutable staging, which is what the relay should run:

- `/Users/marcelo/Library/Caches/rustuml-grok-g2/g2a/reference-manifest.json` sha256 `54189a17206561e9c17cfe7454476391606d585b6c29846584d0638f00096d53`
- `/Users/marcelo/Library/Caches/rustuml-grok-g2/g2a/reference-input-bundle.json.gz` sha256 `3e874ff144c1c929dd2c9a3765777a1352149219dd79fa4e4f8c8d5c69e83481`
- `/Users/marcelo/Library/Caches/rustuml-grok-g2/g2a/rust-predictions.json` sha256 `428d2f989026a651f85a91c8d182b99e15c7c5a9d918d16653f075c5c5ab4474`

Binaries: base `fdec459c22ab0beafe7fee3303be856f4e2d348690424a520fb8f8dcc0393c48`, tip `f48c4a607dab6b31c193ed231c86189224d8c661524e77157620112e88db7abf`.

Every case is intended valid. cwd `PRIVATE_CWD`, timezone `GMT+11:00`, invocation `/Users/marcelo/rustuml-reference/bin/reference --cwd PRIVATE_CWD --timezone GMT+11:00 -- -pipe -tsvg`, env `{}`.

Java revision `71806a23780b04a5ccde2f8ceb5121edad5eb711`, jar `aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053`.

Tip textLengths already recorded, not claimed against Java: plain and italic `Width of 'W'` / `50% off` are `73.8008` / `47.4756` (base `74.6895` / `47.4688`). Bold `buzz kink` / `lazy quiz` are `63.0068` / `58.3447` (base `67.6416` / `61.4482`). Alice, Verdana, Arial and SansSerif are byte-identical to base. The `<style>` bold-italic probe is also byte-identical: that path still does not apply bold (the separate T14.46 note).
