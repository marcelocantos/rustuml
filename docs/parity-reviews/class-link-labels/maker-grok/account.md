# Class link labels (🎯T14.44) — pre-draft account

Maker: grok. Base `cb550c0d67cc729ce764895005b7e86310c9f313`.
Java pin `71806a23780b04a5ccde2f8ceb5121edad5eb711`.
This account is the generative model and the measurements taken before the
draft. It is not a verdict.

The six residues are ordinary Svek link labels. `LinkConstraint.drawMe`
(`LinkConstraint.java` 82–102) draws a constraint-line caption from
`FontParam.ARROW` at the midpoint. None of the failing cases is a constraint
link, so that method is cited and not ported.

## Java construction

`GraphvizImageBuilder` 229–241 builds one `FontConfiguration` per link:

- label font: `getDefaultStyleDefinitionArrow(link.getStereotype())`
  (116–121: `root, element, styleName, arrow`, then `withTOBECHANGED`)
  merged with `link.getStyleBuilder()`, then `Style.getFontConfiguration`
- cardinality font: `getStyleArrowCardinality` (124–130: the same signature
  plus `cardinality`)

`Style.getUFont` (203–214) reads `FontName`, `FontSize` (`asInt`, and the
method's own −1 fallback of 14), `FontStyle.asFontFace()`, then
`FontWeight` only when its CSS weight is not 400 (`withWeight`, italic
preserved). `getFontConfiguration` (217–228) is the font the `TextBlock`
both measures and draws. The builtin arrow skin already supplies size 13;
the existing `unwrap_or(13)` stays. The 14 fallback is not introduced.

`SvekEdge` constructor 279–349:

- null label → empty block, plus `StringWithArrow.addMagicArrow` when the
  link arrow is not `NONE_OR_SEVERAL`
- otherwise `Display.create0(..., CreoleMode.SIMPLE_LINE)` or
  `addSeveralMagicArrows` when `hasSeveralGuideLines` (`Display.java`
  710–721; that predicate is magic-arrow marks on a split line, not the
  newline split itself)
- `addVisibilityModifier` (356–366): margin 6 on a self link, else 1
- quantifiers and roles (`getQuantifier1/2`, `getRole1/2`) are
  `Display.getWithNewlines(...).create(cardinalityFont, CENTER, ...)`
  (322–344). Head/tail text wins over the role when both exist (440–460)

`Display.getWithNewlines` (258–300) splits the two-character sequences
`\n`, `\r` and `\l` (legacy backslash pragma, raw mode off). Horizontal
alignment of a class link is `getDefaultTextAlignment(CENTER)` (369–373).
`appendTable` (497–516) truncates width and height with `(int)`.
`drawU` (944–971) paints `labelText` at `labelXY + labelShield` and the
endpoint blocks at their solved positions. `TextBlockArrow2` (50–77): the
square is `font.getSize2D()`; the triangle is `(int)(size * 0.80)`.

`CommandLinkClass.executeArg` (161–245) reads optional `STEREOTYPE`
(`StereotypePattern.optional`, `StereotypePattern.java` 45–69:
`(\<\<.+?\>\>)`) after the second entity and before `LABEL_LINK`. When
present, `Stereotype.build` + `link.setStereotype` (237–240). The
stereotype is not a drawn label. `removeChevronBrackets` (66–69) strips
one pair of guillemets.

Ortho center labels are `xlabel` (`SvekEdge.appendLine` 423–431,
`DotSplines.ORTHO`) and the painted origin is `getXY` of that table
(734–741). This draft does not change the synthesised ortho inset.

## Precommitted measurements

Relay Java stdout, batch `shared-svg-round9-c9a`, whole SVG:

| Case | Java anchor |
|---|---|
| `c9a_af_size9` | viewBox `0 0 84 190`; label font-size 9, textLength 40.043, x 28.83, y 94.7012 |
| `c9a_af_style_italic` | viewBox `0 0 102 195`; italic, textLength 57.8398 (same advance as plain Sans) |
| `c9a_af_style_bold` | viewBox `0 0 107 195`; textLength 62.3848 |
| `c9a_af_cn_italic_mark` | viewBox `0 0 115 194`; Courier New italic textLength 70.2114; width delta vs control 7.8012 |
| `c9a_af_multiline_vd` | viewBox `0 0 162 211`; Verdana 13 “Pier” textLength 24.6987 x 75.4377 y 99.0698; “quay much longer” textLength 117.9141 x 28.83 y 114.8691. Baseline delta 15.7993 = 13 × 1.21533203125. Shorter line is centered on the longer line |
| `c9a_af_roles_size16` | viewBox `0 0 141 198`; center and the quoted quantifiers “1” / “many” are font-size 16. Source is `A "1" --> "many" B`, not a `/role` |
| `c9a_af_stereo_skinparam` | viewBox `0 0 133 203`; font-size 20, textLength 88.9844; no `<<use>>` text. Diagram type CLASS |
| `c9a_af_ortho_vd` | viewBox `0 0 104 178`; Verdana 13 textLength 61.0962, x 7, y 82.1098. Path `M69,55.24 C69,73.02 69,91.08 69,108.84` |

Size 9, predicted before the draft. SansSerif height at 9 is
`9 * 1.177734375 = 10.599609375` (`plantuml_metrics::text_height`; sizes
10–14 are the only special cases). Plus margin 1×2 = 12.599609375.
`appendTable` `(int)` is 12. The current center layout does
`label_height.max(LINK_ARROW_BLOCK_SIZE)` with the constant 13 even when
`label_arrow` is `None`, then adds 2, so the table is 15. Canvas
195 → 193 (Rust) versus 195 → 190 (Java) is that 3px table difference.
The arrow square is applied only when a magic arrow exists, and then it
is the resolved font size. At size 13, `(int)(13 * 0.80)` stays 10, so
default guide-arrow goldens stay put.

Ortho, predicted before the draft, and not fitted. Both Verdana 13 and
Sans 13 floor to table height 17 (`15.79931640625 + 2` and
`15.310546875 + 2`). Horizontal placement already matches: text ends at
`7 + 61.0962 = 68.0962`, just left of the edge at x=69; box width
truncates to 63, left = 69−63 = 6, text x = left + margin 1 = 7.
Vertical: shortened-path midpoint `(55.24 + 108.84) / 2 = 82.04`.
Current synthesizer (`y = midpoint − height + 5`, then draw
`+ margin + MARGIN + Verdana ascent 13.06982421875`) produces baseline
84.1098. Java is 82.1098. The delta is exactly 2, for any face whose
table height floors to 17. `ORTHO_XLABEL_VERTICAL_INSET` stays 5.0 in
this draft. Seventy-one goldens use `linetype ortho` and fifty-six of
those also contain `:`. The inset was extracted for those layouts.
Changing 5 to 3 from this one SVG would move them. `ORTHO_ER_VERTICAL_ROUTE_DELTAS`
is not touched. A later xlabel port (`appendLine` 427–429 plus `getXY`
734–741, and only a class-edge flag — layout currently emits `label`,
and `rustuml_edge_label_box` reads `ED_label` only) requires an addendum
before any placement edit.

## Draft that this account authorises

- `Relationship.stereotypes`: inner text of one `<<...>>` between the
  target and the label (`CommandLinkClass` 237–240). Other constructors
  store an empty list. The token is not drawn. A `<<...>>` after the
  colon stays a label (`normalize_relationship_label` already converts it).
- Arrow and cardinality signatures gain `with_stereotype` for each stored
  label (`GraphvizImageBuilder` 116–130, 235–238).
- Center and endpoint measure/draw use that font's family, size, and
  `font_face(FontStyle)`. `FontWeight` overrides only the weight axis
  when the property is present and is not CSS 400 (`Style.getUFont`
  209–212). `bold italic` stays plain, because `asFontFace` says so.
- Center labels split on the two-character `\n` the same way endpoint
  quantifiers already do. Width is the max line. Height is the sum of
  line heights. Each line is centered on that max width. `\r` / `\l`
  alignment is not in this draft.
- Magic-arrow square and triangle use the resolved font size
  (`TextBlockArrow2` 57–63). No magic arrow → no `.max(square)` on a
  text label.
- Endpoint Graphviz boxes and SVG use the cardinality font
  (`SvekEdge` 322–331, 440–459). Quoted multiplicities are already parsed;
  no role-syntax parser is added.
- Ortho inset 5.0 is unchanged. The ortho box uses the same text metrics
  so a multiline or non-default face sizes the synthesised rectangle;
  the y formula is not retuned.

`c9a_af_usecase_vd`, `c9a_af_deploy_size16`, `c9a_af_component_vd` and
`c9a_af_object_vd` are outside this file's link path and are not edited.
`c9a_af_state_vd` already matches; `state.rs` is not edited.
