# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Independent checker matrix; raw Java CLI-pipe references, never rewrite old fixtures."""
from pathlib import Path
import concurrent.futures
import hashlib
import json
import os
import subprocess

ROOT = Path.cwd()
OUT = Path('/tmp/rustuml-checker2740')
SOURCES = OUT / 'new-sources'
CASES = []

def case(mechanism, name, body, axes):
    source = SOURCES / mechanism / (name + '.puml')
    source.parent.mkdir(parents=True, exist_ok=True)
    source.write_text('@startuml\n' + body.rstrip() + '\n@enduml\n')
    CASES.append({'mechanism': mechanism, 'name': name, 'source': str(source), 'axes': axes, 'origin': 'fresh independent checker'})

def add(short, name, body, *axes):
    case('class-' + short, name, body, list(axes))

# Every matrix varies at least two causal axes, rather than fixture-label mutations alone.
for i, (kind, display) in enumerate([('class', 'actor actor actor'), ('record', '[network] as database'), ('INTERFACE', 'component usecase queue')]):
    add('command-execution-ownership', f'opaque_root_{i}', f'{kind} "{display}" as Ledger\nclass Peer\nLedger --> Peer', 'quoted command-looking display', 'native type and root topology')
for align in ['left', 'right']:
    add('command-execution-ownership', f'body_{align}', f'{align} header\nallowmixing\ncomponent Ghost\n\nclass Phantom\nend header\npackage Vault {{\nclass Actual {{\nallowmixing\ncomponent field\n}}\n}}', 'aligned multiline header with internal blank', 'class member command ownership')
for i, prefix in enumerate(['allowmixing\n', '']):
    add('command-execution-ownership', f'mixed_order_{i}', f'{prefix}package Dock {{\nclass Native\n' + ('component' if prefix else 'mix_component') + ' "Full widget" as Widget\nNative --> Widget\n}', 'allowmixing versus mix prefix', 'native and description declaration order')

for i, (kind, decl) in enumerate([('node', '"Windward Harbor" as Port'), ('frame', 'Port as "Windward Harbor"'), ('cloud', '"Small" as Port'), ('folder', 'Port as "Wide Repository"')]):
    add('symbol-container-alias-identity', f'alias_{kind}', f'{kind} {decl} {{\nclass Inside\n}}\n{kind} Port {{\nclass Next\nInside --> Next\n}}', 'forward/reverse display and code', 'reopened container and kind')
for i, (kind, visibility, label) in enumerate([('record','+','Long dossier'),('protocol','-','P'),('struct','#','Ledger\\nEntry'),('metaclass','~','Schema'),('exception','+','Overflow'),('dataclass','-','Coordinates')]):
    add('native-leaf-presentation', f'native_{kind}', f'skinparam classAttributeIconSize {8 if i % 2 else 14}\nnode "Native types" as N {{\n{visibility}{kind} "{label}" as V {{\n+id: int\n}}\n}}', 'native kind and visibility/icon size', 'multiline or wide label and body')
for i, side in enumerate(['left','right','top','bottom']):
    add('note-target-quark', f'target_{side}', f'package "Outer harbor" as H {{\nclass "Air sensor" as S\nnote {side} of S\nReading {i}\nsecond line\nend note\n}}', 'scoped alias target', 'note side and multiline body')
for i, name in enumerate(['Air Probe', 'Air_Probe', '<b>Wind</b> Probe', 'unit as record']):
    add('quoted-code-identity', f'quoted_{i}', f'package "Identity scope" {{\nclass "{name}"\nclass Peer\n"{name}" --> Peer\nnote right of "{name}" : audit\n}}', 'spaces/markup/command text in quoted code', 'relationship and attached-note lookup')

for i, kind in enumerate(['component','boundary','actor','database','queue','rectangle']):
    add('command-grammar-closure', f'mixed_{kind}', f'allowmixing\nnode Dock {{\nclass Native\n{kind.upper() if i % 2 else kind} "External {kind}" as Ext\nNative --> Ext\n}}', 'mixed leaf kind and keyword case', 'nested container and explicit alias')
for i, symbol in enumerate(['component','boundary']):
    for mode in ['uml1','uml2']:
        add('description-symbol-images', f'{symbol}_{mode}', f'skinparam componentStyle {mode}\nallowmixing\nclass Local\n{symbol} "Remote\\nService" as Remote <<gateway>> #LightBlue\nLocal --> Remote', 'symbol image and UML style', 'multiline/stereotype/color and connected native peer')
for kind in ['action','process','queue']:
    add('description-symbol-images', f'{kind}_group', f'{kind} "Delivery Pipeline" as Work <<engine>> {{\nclass Worker\n}}\nclass Client\nClient --> Work.Worker', 'container symbol and stereotype', 'wide alias and external link')

for i, (kind, themes) in enumerate([('class',('cerulean','minty')),('interface',('minty','cerulean')),('record',('cerulean','minty')),('struct',('minty','cerulean'))]):
    add('current-spot-cascade', f'theme_{kind}', f'!theme {themes[0]}\n{kind} "Early declaration" as Earlier\n!theme {themes[1]}\n{kind} "Late" as Later\nEarlier --> Later', 'theme order and declaration epoch', 'native spot type and label width')
for i, placement in enumerate(['before','after']):
    override = 'skinparam stereotypeCBackgroundColor #1278ab\nskinparam stereotypeCBorderColor #bb4411\n'
    body = ('!theme minty\n' + override if placement == 'before' else '!theme minty\n') + 'class Earlier\n!theme cerulean\n' + (override if placement == 'after' else '') + 'class Later\nEarlier --> Later'
    add('current-spot-cascade', f'override_{placement}', body, 'explicit override position relative to themes', 'declaration epoch')
for kind in ['class','record']:
    add('current-spot-cascade', f'custom_{kind}', f'!theme cerulean\n{kind} "Custom spot" as First <<(Z,#123456) marker>>\n!theme minty\n{kind} Second\nFirst --> Second', 'custom spot versus native default', 'theme change after declaration')

for i, align in enumerate(['left','center','right']):
    for wide in [False, True]:
        label = 'Long and wide decorated graph title' if wide else 'T'
        add('chrome-bordered-block-geometry', f'{align}_{wide}', f'title {label}\n{align} header\nHeader\nsecond line\nend header\n{align} footer Footer\ncaption Caption\nlegend {align}\nKey\nend legend\nnode Box {{\nclass Payload\n}}', 'header/footer/legend alignment', 'title width and complete chrome stack')
for i, side in enumerate(['left','right','top','bottom']):
    outside = i % 2 == 0
    body = 'frame Site {\nclass Payload\n' + ('' if outside else f'note {side} of Payload : in scope\n') + '}\n' + (f'note {side} of Site.Payload : root scope\n' if outside else '')
    add('note-declaration-scope', f'scope_{side}', body, 'root versus target declaration scope', 'attachment side and frame frontier')
for i, kind in enumerate(['cloud','node','rectangle','process']):
    add('svek-painted-origin', f'origin_{kind}', f'{"left to right direction" if i % 2 else "top to bottom direction"}\n{kind} "Variable width envelope" as Env {{\nclass "Small" as A\nclass "Wider child label" as B\nA --> B\n}}', 'painted container shape and direction', 'child label widths and linked topology')
for i, kind in enumerate(['node','frame','action','queue']):
    add('group-paint-ownership', f'url_{kind}', f'skinparam svgLinkTarget _blank\n{kind} "Outer {kind}" as O [[https://example.test/outer{{Outer tooltip}}]] {{\nrectangle "Inner" as I [[https://example.test/inner{{Inner tooltip}}]] {{\nclass Leaf\n}}\n}}\nclass Peer\nO.I.Leaf --> Peer', 'nested URL and symbol ownership', 'root peer and cross-group link')
for i, arrow in enumerate([')-( ', ')-- ', '--( ', ')-left-( ']):
    add('parenthesis-link-identity', f'endpoint_{i}', f'package Lab {{\nclass "A long endpoint" as A\nclass B\nA {arrow}B\n}}', 'source/target/double/inverted decorations', 'scoped aliases and width')
for i, decl in enumerate(['package "Harbor" <<first>> as Hub <<last>>', 'node "Harbor" <<first>> as Hub <<last>>', 'package Hub as "Harbor" <<one>> <<two>>', 'node Hub as "Harbor" <<one>> <<two>>']):
    add('package-command-precedence-closure', f'slots_{i}', f'frame Outer {{\n{decl} {{\nclass Payload\n}}\n}}', 'ordinary versus symbol grammar', 'pre/post alias stereotype slots and nesting')
for i, (direction, arrow) in enumerate([('TB','-->'),('LR','-->'),('TB',')-('),('LR',')-('),('TB',')--'),('LR','--(')]):
    directive = 'left to right direction\n' if direction == 'LR' else ''
    add('group-endpoint-routing', f'groups_{i}', directive + f'package "First shipping group" as G {{\nclass Item\n}}\npackage "Destination" as H {{\nclass Receiver\n}}\nG {arrow} H', 'group source/target/both decorations', 'LR/TB and label width')
for i, kind in enumerate(['package','node','cloud','queue']):
    add('container-only-factory-closure', f'empty_{kind}', f'{kind} "Empty endpoint" as E [[https://example.test/resource{{A tooltip}}]] {{\n}}\n{kind} "Other endpoint" as F {{\n}}\nE --> F', 'empty container kind and URL', 'two linked aliased containers')
for i, (before, nested) in enumerate([(False,False),(True,False),(False,True),(True,True)]):
    lines = 'class Peer\n'
    link = 'N .. Peer\n'
    note = 'note "Annotation with width" as N\n'
    body = lines + (link + note if before else note + link)
    if nested:
        body = 'package Harbor {\n' + body + '}\n'
    add('named-note-scoped-identity-closure', f'named_{i}', body, 'link before/after named note', 'root/nested quark ownership')
for i, (title, direction, arrow) in enumerate([('N','TB',')--'),('Long folder shoulder title','TB','--('),('N','LR',')-('),('Long folder shoulder title','LR',')--')]):
    add('group-magnetic-extremities', f'magnetic_{i}', ('left to right direction\n' if direction == 'LR' else '') + f'folder "{title}" as F {{\nclass Inside\n}}\nclass Peer\nF {arrow} Peer', 'folder shoulder width and direction', 'endpoint side and group-to-leaf route')
for i, (title, stereo) in enumerate([('N','Extremely wide stereotype label'),('A very long node title','s'),('N','one>> <<another long label'),('N','s')]):
    outer = 'package Nest {\n' if i % 2 else ''
    add('cluster-header-shape-extent', f'header_{i}', outer + f'node "{title}" as Host <<{stereo}>> {{\nclass Leaf\n}}\n' + ('}\n' if outer else ''), 'title versus stereotype width/multiple labels', 'direct/nested node alias')
for i, (kind, side) in enumerate([('cloud','left'),('node','right'),('rectangle','top'),('action','bottom')]):
    add('single-svek-envelope-origin', f'root_note_{kind}', f'{kind} "Outer envelope" as G {{\nframe Inner {{\nclass Payload\n}}\n}}\nclass Peer\nPeer --> G.Inner.Payload\nnote {side} of G.Inner.Payload : root annotation', 'nested shape frontier with root peer', 'root note side and cross-group spline')

# Earlier dependency accounts are explicitly superseded by concrete continuation accounts.
for dependency, successor in [('cloud-child-coordinate-dependency','svek-painted-origin'),('description-leaf-image-dependency','description-symbol-images'),('note-cluster-ownership-dependency','note-declaration-scope'),('page-chrome-composition-dependency','chrome-bordered-block-geometry')]:
    originals = [c for c in CASES if c['mechanism'] == 'class-' + successor][:2]
    for original in originals:
        source = Path(original['source']).read_text().split('\n',1)[1].rsplit('@enduml',1)[0]
        add(dependency, 'dependency_' + original['name'], source, *original['axes'])
        CASES[-1]['origin'] = 'checker successor probe retained for dependency mapping'

add('command-execution-ownership', 'negative_late_mixing', 'package G {\nclass X\ncomponent C\nallowmixing\n}', 'late mixing directive', 'nested native/description order')
add('native-leaf-presentation', 'negative_mandatory_modifier', 'node N {\n*class Required\n}', 'invalid declaration visibility', 'container context')

def generate(item):
    source = Path(item['source'])
    command = ['/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java', '-Djava.awt.headless=true', '-jar', '/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar', '-tsvg', '-pipe']
    env = dict(os.environ, JAVA_HOME='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home')
    env['PATH'] = env['JAVA_HOME'] + '/bin:' + env['PATH']
    result = subprocess.run(command, input=source.read_bytes(), capture_output=True, env=env, timeout=90)
    source.with_suffix('.svg').write_bytes(result.stdout)
    source.with_suffix('.java.stderr').write_bytes(result.stderr)
    return {**item, 'java_command': command, 'java_exit': result.returncode, 'source_sha256': hashlib.sha256(source.read_bytes()).hexdigest(), 'java_sha256': hashlib.sha256(result.stdout).hexdigest()}

if __name__ == '__main__':
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        results = list(pool.map(generate, CASES))
    (OUT / 'generation.json').write_text(json.dumps(results, indent=2) + '\n')
    print(f'Preserved {len(results)} Java pipe sources/references; nonzero Java exits: {sum(r["java_exit"] != 0 for r in results)}')
