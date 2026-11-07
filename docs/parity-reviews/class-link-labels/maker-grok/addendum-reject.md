# Class link labels — correction before the rejected-round fix

Written after checker review `2d7f0dcfa4c24489788749e00d155eac3e6567fa`
and before the code change. Batch CO is
`/Users/marcelo/Library/Caches/rustuml-relay-parent/grok-g4-checker-heldout/reference-results.json`
SHA256 `74b934fd` prefix, 100 cases. This note is not a verdict.

## What Java does

`CommandLinkClass` builds the label with `Display.getWithNewlines`
(`CommandLinkClass.java` 281, 316, 353). The loop at
`Display.java` 258–310 turns raw mode on at `[[`, `<math>`, and `<latex>`,
and off at the matching close. Outside raw mode a backslash is an escape:
`\n`, `\r`, and `\l` break the line, `\r` and `\l` set the natural
alignment, `\t` is a tab, `\\` is one backslash, and any other pair is
kept. `JawsFlags.SPECIAL_NEWLINE_IN_DISPLAY_CLASS` is false, so `%n()` is
not this loop. `create0` then replaces the requested alignment with that
natural alignment (`Display.java` 644–645).

A creole URL is one display line while raw mode holds. `AtomTextUtils.createUrl`
(`AtomTextUtils.java` 79–88) calls `getWithNewlines` again on
`Url.getLabel`, and a label of more than one line becomes
`AtomVerticalTexts` (`AtomVerticalTexts.java` 53–74): the box width is the
widest line, later lines start at the left of that box, and each line is
its own link to the same URL. CO `g4k_nl_url` is that shape. Java draws
`pre`, then `http://example.com/a` and `b` in blue underline at x 42.86,
then `post`. Both anchors use href `http://example.com/a\nb` and a title
with `&#10;`. ViewBox `0 0 201 241`.

CO `g4k_nl_escaped` is source `a\\nb`. The `\\` escape leaves one line
whose text is `a\nb`. ViewBox `0 0 91 195`.

`SvekEdge.appendLine` sends an orthogonal center label as `xlabel`.
`normalize_ortho_er_vertical_route` rewrites a DoubleLine-to-CircleCrowFoot
spline only when every point shares x. CO `g4k_ortho_dl_cc_lr` is
horizontal, so that rewrite does not run. Java keeps the classes at
y 28.291 and draws `firing` / `shelf` at y 44.5684 and 59.8789.
ViewBox `0 0 209 78`. The reverse `}o--||` case matches today.

## What the current Rust does

`relationship_label_lines` is `split("\\n")`, so `a\\nb` becomes `a\` and
`b`, and a `\n` inside `[[...]]` is cut before creole sees the URL.
`ortho_route_rewritten` is true for every DoubleLine-to-CircleCrowFoot
edge, so the horizontal case never receives an xlabel and the synthesizer
moves the classes to y 35.291.

## Planned change

Split link labels with the `Display.getWithNewlines` escape and raw-mode
rules above. A link segment whose label still contains `\n` is stacked
like `AtomVerticalTexts`. Send an xlabel for every orthogonal center
label, and clear it only after `normalize_ortho_er_vertical_route`
actually rewrites the points, so the synthesizer runs only on that route.
Leave `<style> arrow <<name>>` and `g4k_q_empty` alone.
