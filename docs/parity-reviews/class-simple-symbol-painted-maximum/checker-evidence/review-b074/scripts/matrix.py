# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64, gzip, hashlib, json
from pathlib import Path
S = Path('/Users/marcelo/Library/Caches/rustuml-review-b074')
D = S / 'sources'; D.mkdir(exist_ok=True)
rows = []
def add(mechanism, name, body, axes):
    data = ('@startuml\n' + body + '\n@enduml\n').encode()
    p = D / (name + '.puml'); p.write_bytes(data)
    rows.append(dict(name=p.name, source=str(p), source_sha256=hashlib.sha256(data).hexdigest(), source_base64=base64.b64encode(data).decode(), timezone='GMT+11:00', resources=[], mechanism=mechanism, axes=axes))
for symbol in ['actor','boundary']:
    cases = [
        ('plain', 'Observatory', 17, '', '', '', 'linked'),
        ('multiline', 'North pier\\nSignal mast\\nTerminal', 25, ' <<watch>>', 'left to right direction', 'FontStyle bold\nHorizontalAlignment right', 'linked'),
        ('sized', '<size:31>Signal</size>\\n<sub>tide</sub>', 13, '', '', '', 'linked'),
        ('blanktail', 'Observatory\\n', 19, '', '', '', 'linked'),
        ('empty', '', 9, '', 'left to right direction', '', 'linked'),
        ('shadow', 'i', 7, '', 'skinparam shadowing true', 'Shadowing 18\nLineThickness 5', 'linked'),
        ('subgroup', 'Tide\\nClock', 21, ' <<watch>>', 'left to right direction', 'LineThickness 3', 'subgroup'),
        ('degenerate', 'Signal\\nPier', 17, ' <<watch>>', '', '', 'single'),
    ]
    for tag,label,size,stereo,direction,sty,topology in cases:
        declaration = f'{symbol} "{label}" as Beacon{stereo}'
        if topology == 'linked': declaration = 'class Station\n' + declaration + '\nStation --> Beacon'
        if topology == 'subgroup': declaration = 'package Harbor {\npackage Channel {\n'+declaration+'\n}\n}\nclass Station\nStation --> Beacon'
        add('class-simple-symbol-painted-maximum', f'{symbol}_{tag}', f'allowmixing\n{direction}\n<style>\n{symbol} {{\nFontSize {size}\n{sty}\n}}\n</style>\n{declaration}', ['symbol',tag,'font size','topology'])
for group in [False,True]:
    for i,pos in enumerate(['left','top','right','bottom']):
        braced = i % 2 == 0
        target = 'Harbor' if group else 'Station'
        declaration = 'package "Harbor office" as Harbor {\nclass Station\n}' if group else 'class Station'
        url = f' [[https://example.org/tide?port=7&day=2{{Inspect {pos}}}]]'
        body = f'skinparam svgLinkTarget {"_self" if braced else "_blank"}\n{declaration}\nnote {pos} of {target} $watch <<memo>> #LightYellow{url}{" {" if braced else ""}\nObservation {pos}\nclass is prose\n'+('}' if braced else 'end note')
        add('class-note-url-group-ownership', f'note_{"group" if group else "leaf"}_{pos}', body, ['target kind','position','delimiter','URL target/tooltip'])
for linked in [False,True]:
    for url in [False,True]:
        declaration = 'class Station\nnote "Detached memorandum" as Memo'
        if url: declaration += ' [[https://example.org/memo{Read memorandum}]]'
        if linked: declaration += '\nMemo .. Station'
        add('class-note-url-group-ownership', f'note_named_{linked}_{url}', 'skinparam svgLinkTarget _parent\n'+declaration, ['named floating note','link topology','URL present/absent'])
raw = (json.dumps({'batch':'renderer-review-b074-independent28','inputs':rows},separators=(',',':'))+'\n').encode()
packed = gzip.compress(raw,mtime=0)
(S/'inputs.json').write_bytes(raw); (S/'inputs.json.gz').write_bytes(packed)
(S/'manifest.json').write_text(json.dumps([{k:v for k,v in r.items() if k!='source_base64'} for r in rows],indent=2)+'\n')
(S/'transport.json').write_text(json.dumps({'sha256':hashlib.sha256(packed).hexdigest(),'data':base64.b64encode(packed).decode()})+'\n')
print(len(rows),hashlib.sha256(packed).hexdigest())
