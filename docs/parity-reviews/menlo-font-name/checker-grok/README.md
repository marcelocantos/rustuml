# Checker record: menlo-font-name

Verdict: **rejected**. The packet is not accept-ready for a local merge.

Reviewed tip `11d1edbf8948ff660b98bba210fda0990f5b5b75`. Account `bf5fc5b94dd24682d94e3291f7a6dc57700c47a1` precedes the port `6d17122b73f87599a6a0e64fcb8f67046dc4993f`. Base is `cb550c0d67cc729ce764895005b7e86310c9f313`. The only crate diff is `crates/rustuml-render/src/text_render.rs`.

Java evidence is the immutable relay. Batch CN results SHA256 `7d1c1dadc323875f15b5ed15bb06a84a238bb57f1a279e1f7d0dc4bc16efdcd1` (46/46 exit 0, empty stderr). The AWT probe stdout SHA256 is `dd457c41021bf92545e0c4e7c5ca897fba4815f30e79968bdc0a3855f59a37ce`. Counts are in `scoreboard.json`.

## Why it fails

`Font.decode("monospace")` is Dialog (`51.966796875`). `Font.decode("Monospaced")` is Monospaced (`67.4296875`). `modelled_family` maps both, plus the SVG spelling, to Menlo. `skinparam defaultFontName monospace` is unchanged by the packet and still measures Menlo.

`<font:monospace>` on a Menlo surrounding face measured Dialog at the base (`textLength 51.9668`, Java-equal) and Menlo at the tip (`67.4297`). The surrounding-face exception is the creole recovery added in `23982a58f`. It matches the activity golden only because that golden's surrounding face is Dialog.

`skinparam sequenceFontName` is not a `FontParam`. Java keeps Times metrics (`viewBox 0 0 112 123`). The tip resolves `Helvetica-Bold` and changes the geometry to `viewBox 0 0 109 115`.

## What did match

Batch CG is 6/6 under `compare_svg_strict` at the tip and 1/6 at the base. Batch CN is 29/46 at the tip and 8/46 at the base, with 21 gains and no lost whole-SVG match. All eight `c9a_ns_menlo_*` controls become strict matches. CLI stdout is byte-identical on 12474/12550 goldens; the other 76 exit 1 on both trees with identical streams. Of 6328 perturbations, 14 stdout values change, 13 of those become strict matches, and none stop matching.

`golden_no_oracle` is 11181/11251 on both trees, and the 70 failure names are byte-identical.

## Guard

`parity_review_records_are_complete` stays red. This review is the latest for `menlo-font-name`, its verdict is `rejected`, and four of its perturbations fail. The guard requires the latest review to be accepted and to contain no failing perturbation.
