# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Source-guided counterexamples; retain the entire first matrix unchanged."""
from pathlib import Path
import concurrent.futures
import json
import runpy

g = runpy.run_path(str(Path(__file__).with_name('checker2740_generate.py')))
g['CASES'].clear()
add = g['add']

for i, kind in enumerate(['record','struct']):
    add('command-execution-ownership', 'bare_native_' + kind, f'{kind} "Root declaration" as R\n{kind} Peer\nR --> Peer', 'new native kind', 'root aliased declaration plus relationship')
for i, kind in enumerate(['class','interface']):
    add('command-execution-ownership', 'braced_note_' + kind, f'node Scope {{\n{kind} Item\nnote right of Item {{\ncomponent Shadow\nclass Phantom\n}}\n}}', 'brace-delimited multiline note ownership', 'native type and nested scope')
for prefix in ['+','-']:
    add('package-command-precedence-closure', 'visibility_' + ('public' if prefix=='+' else 'private'), f'{prefix}package "Visible package" as P <<domain>> {{\nclass Payload\n}}', 'ordinary package visibility', 'alias plus stereotype')
for side in ['left','bottom']:
    add('note-target-quark', 'group_target_' + side, f'node "Group target" as G {{\nclass Item\n}}\nnote {side} of G : group annotation', 'group versus leaf quark target', 'attachment side')
for i, decl in enumerate(['note "Direct root" as H.N', 'note as H.N\nMultiline body\nend note']):
    add('named-note-scoped-identity-closure', f'qualified_{i}', f'package H {{\nclass Peer\n}}\n{decl}\nH.N .. H.Peer', 'qualified named-note identity', 'single/multiline declaration')
for size in [0,9,14]:
    add('native-leaf-presentation', f'bodyless_icon_{size}', f'skinparam classAttributeIconSize {size}\nnode Group {{\n+record "Public declaration" as R\n~protocol P\nR --> P\n}}', 'visibility icon size including odd and zero', 'native kind and bodyless declaration')
for i, kind in enumerate(['record','struct']):
    add('current-spot-cascade', 'grouped_' + kind, f'!theme cerulean\nnode Group {{\n{kind} Early\n!theme minty\n{kind} Later\nEarly --> Later\n}}', 'native spot type', 'theme change within container scope')
for kind in ['class','record']:
    add('current-spot-cascade', 'custom_grouped_' + kind, f'!theme cerulean\nnode Group {{\n{kind} Early <<(Z,#123456) marker>>\n!theme minty\n{kind} Later\nEarly --> Later\n}}', 'custom spot and native type', 'container plus later theme')
for align in ['left','right']:
    add('chrome-bordered-block-geometry', 'style_' + align, f'<style>\nheader {{\n HorizontalAlignment {align}\n FontSize 18\n}}\n</style>\nheader\nWide header line\nShort\nend header\nclass Anchor', 'current style alignment/font size', 'multiline unequal-width header')
for kind in ['folder','node']:
    add('symbol-container-alias-identity', 'relabel_' + kind, f'{kind} "First display" as G {{\nclass Earlier\n}}\n{kind} "Second unrelated display" as G {{\nclass Later\nEarlier --> Later\n}}', 'reopening same alias with new display', 'container kind and declaration order')
for kind in ['component','boundary']:
    add('description-symbol-images', 'plain_multiline_' + kind, f'allowmixing\n{kind} "Wide display name\\nTiny" as C\nclass Peer\nC --> Peer', 'unequal multiline label widths', 'mixed kind and declaration order')
for i, direction in enumerate(['TB','LR']):
    add('group-endpoint-routing', 'shield_' + direction, ('left to right direction\n' if direction=='LR' else '') + 'COMPONENT Bridge\nINTERFACE "A much wider interface label" as Port\nBridge ..> Port', 'interface shield label width', 'LR/TB direction and uppercase description fallback')
for i, side in enumerate(['left','top']):
    add('single-svek-envelope-origin', 'single_root_note_' + side, f'frame Envelope {{\nclass Item\n}}\nnote {side} of Envelope.Item : root annotation', 'external note versus group frontier', 'horizontal/vertical image minimum')

with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
    results=list(pool.map(g['generate'],g['CASES']))
(g['OUT']/'generation-followup.json').write_text(json.dumps(results,indent=2)+'\n')
print(f'Preserved {len(results)} additional references, {sum(r["java_exit"]!=0 for r in results)} nonzero Java exits')
