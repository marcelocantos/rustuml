# T14.40 maker account — AtomText character references

Precommitted before the production draft. Base `cb550c0d67cc729ce764895005b7e86310c9f313`. Java revision `71806a23780b04a5ccde2f8ceb5121edad5eb711`.

## Java trace

`AtomText` constructor (`klimt/creole/legacy/AtomText.java` lines 73–91) unhides with `CharHidder.unhide`, then, when `manageSpecialChars` is true, calls in order:

1. `StringUtils.showComparatorCharacters` (not this obligation)
2. `StringUtils.manageAmpDiese` (`StringUtils.java` lines 472–484, pattern line 472)
3. `StringUtils.manageUnicodeNotationUplus` (`StringUtils.java` lines 457–470, pattern line 457)
4. `StringUtils.manageTildeArobaseStart` (lines 486–489; not this obligation)
5. `StringUtils.manageEscapedTabs` (lines 491–493; Rust already rewrites `\t` in `text_render::normalize_tab_escapes` before parsing)

`AtomTextUtils.createLegacy` (lines 71–72) passes `manageSpecialChars=true`. URL literal pieces (`AtomTextUtils.java` lines 104 and 137) and `createListNumber` (line 158) construct `AtomText` with true. `AtomTextUtils.create` (lines 75–76) passes false and is the emoji fallback only.

`manageAmpDiese`: `&#([0-9]+);`, decimal only, semicolon required. `Integer.parseInt` then `Character.toChars`. `Matcher.appendReplacement` is called **without** `quoteReplacement`, so a replacement of `$` (`&#36;`) or `\` (`&#92;`) throws and the constructor does not publish a partial string. Surrogates and values above U+10FFFF throw from `toChars`. Values that do not fit in a signed 32-bit int throw from `parseInt`. One pass: replacements are not scanned again.

`manageUnicodeNotationUplus`: `\<U\+([0-9a-fA-F]{4,5})\>`. `U` is uppercase. Four or five hex digits only, so `<U+10FFFD>` (six digits) does not match. `appendReplacement` uses `Matcher.quoteReplacement`, so `$` and `\` succeed. `toChars` still throws for surrogates and for values above U+10FFFF (`<U+110000>` through `<U+FFFFF>`).

Order is amp, then uplus. `&#60;U+0041>` becomes `A`. `&#38;#65;` becomes the characters `&#65;`. `<U+0026>#65;` becomes the characters `&#65;` because amp has already run.

## Precommitted measurement

Walk of `test-diagrams/golden` at submodule `9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a`: 12,550 `.puml` sources, 0 containing `<U+`, 0 containing `&#[0-9]+;`. The golden corpus cannot move because a source token of either form was rewritten.

At this base, `creole::push_literal` restores hidden characters and XML-escapes the result. It does not run `manageAmpDiese` or `manageUnicodeNotationUplus`, so a literal atom draws the reference characters.

## Planned change

In `push_literal`, after `restore_hidden_characters` and before `escape_creole_text`, apply amp then uplus to that one literal atom. An exception-equivalent match (int overflow, `toChars` rejection, or an unquoted `$`/`\` amp replacement) leaves the restored string unchanged. No second scan. No fixture key.

Tooltip text stripped by the `[[url{tooltip} label]]` walk stays on `normalize_svg_link_title`. It is not an AtomText.

`DriverTextSvg` / the SVG transformer writes non-ASCII text-node scalars as decimal `&#N;` (US-ASCII), the same rule `escape_attribute` already uses. `write_text_element` will do that after the existing U+00A0 → `&#160;` rewrite. ASCII, including CR, stays raw so `svg_export` still owns `&#13;`. This is not a per-fixture rewrite: every `emit_text` scalar takes the same encoding. Strict XML comparison decodes both forms.

Out of scope, named so they are not silent: `showComparatorCharacters`, `manageTildeArobaseStart`, and a second tab pass. C0 and PUA results that make Java emit a crash page stay T14.20; this change will not invent a crash SVG. Positive probes stay outside U+E000–U+E0FF and C0.

## Axes

- syntax: `<U+XXXX>` versus `&#N;`, including 4 versus 5 versus 6 hex digits and decimal versus `&#x`
- order: amp-produced `<U+…>` is converted; uplus-produced `&#N;` is not scanned again
- throw path: `$`/`\` amp replacements and illegal code points leave the atom literal
- consumer: the same literal atom in class, sequence, activity, and note text
- separator and mark: U+2028, U+2029, U+0301 introduced only by a reference
