# Provisional chain extraction: the italic faces, on colossus

`tools/FontChainExtract.swift` output for the ten italic, bold-italic and oblique faces of the modelled
families that the reference-host extraction (`../monkeyman-round6`) does not cover. It was run on
colossus (macOS 27.2 26B123, the same build as Monkey Man) because JDK measures italic text with these
faces (round-9 maker per-face-facts account) and the composite cascade coverage differs by style
(100-207 code points per face on colossus).

This is not reference-host data: colossus and Monkey Man differ on cascade coverage (T14.6.8; e.g.
PingFang from a MobileAsset on Monkey Man). A Monkey Man extraction of every style was requested in
shared-SVG round 9; when it returns it replaces this directory. Until then `font_chain`'s drift gate
lists these faces in `PROVISIONAL_CHAIN` and fails if the list is stale.
