# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json
from pathlib import Path
C=Path(__file__).parent;P=C/'reference-inputs';P.mkdir();sha=lambda b:hashlib.sha256(b).hexdigest();cases=[]
controls=[
('svg_upper','SPRITE $cairn <svg width="8" height="8">\n<rect width="8" height="8"/>\n</svg>\n',True),
('svg_nbsp','sprite\u00a0$cairn\u00a0<svg width="8" height="8">\n<rect width="8" height="8"/>\n</svg>\n',True),
('svg_inline','sprite $cairn <svg width="8" height="8"><rect width="8" height="8"/></svg>\n',True),
('pixel_upper_end','SPRITE $ridge [2x2/16] {\nFF\nF0\nEND SPRITE\n',True),
('pixel_nbsp_brace','sprite\u00a0$ridge.top [2x2/16] {\nFF\nF0\n}\n',True),
('pixel_compact_end','sprite $ridge [2x2/16] {\nFF\nF0\nendsprite\n',True),
('participant_dotted','participant "High cliff" as Ridge.Top\nparticipant Valley\n',True),
('participant_at_compact','participant @Ridge<<edge>>#Wheat\nparticipant Valley\n',True),
('participant_negative','participant Ridge order -3\nparticipant Valley order 4\n',True),
('participant_composite','participant Ridge #back:Wheat;line:Olive\n',False),
('participant_gradient','participant Ridge #Wheat-Olive\n',True),
('participant_repeat_color','participant Ridge\nparticipant Ridge #NotAColourAtAll\n',True),
('participant_multiline_color','participant Ridge #NotAColourAtAll [\nRidge\n]\n',True),
('note_box_colors','box "Valley" #back:Wheat;line:Olive\nparticipant Ridge\nend box\nnote over Ridge #Wheat-Olive: cargo\n',True),
]
for name,body,valid in controls:
 for teoz in [False,True]:
  n=name+('_teoz' if teoz else '_legacy');p=P/(n+'.puml');p.write_text('@startuml\n'+('!pragma teoz true\n' if teoz else '')+body+'Ridge -> Valley: survey\nValley --> Ridge: logged\n@enduml\n');cases.append(dict(name=n,source=str(p),source_sha256=sha(p.read_bytes()),cwd='PRIVATE_CWD',timezone='GMT+11:00',env={},invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=[name,'teoz' if teoz else 'legacy'],intended_valid=valid))
for family,body in [('class','class Ridge\nclass Valley\nRidge --> Valley\n'),('state','[*] --> Ridge\nRidge --> Valley\n')]:
 n='svg_'+family;p=P/(n+'.puml');p.write_text('@startuml\nSPRITE\u00a0$peak <svg width="8" height="8">\n<rect width="8" height="8"/>\n</svg>\n'+body+'@enduml\n');cases.append(dict(name=n,source=str(p),source_sha256=sha(p.read_bytes()),cwd='PRIVATE_CWD',timezone='GMT+11:00',env={},invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=['SVG multiline ownership',family],intended_valid=True))
m=dict(schema_version=1,purpose='Sprite block consumption, complete participant declarations and command-specific color execution',cases=cases);raw=(json.dumps(m,indent=2)+'\n').encode();(C/'reference-manifest.json').write_bytes(raw);bundle=dict(manifest=m,manifest_sha256=sha(raw),files=[dict(path=p.name,sha256=sha(p.read_bytes()),base64=base64.b64encode(p.read_bytes()).decode()) for p in sorted(P.glob('*.puml'))],resources=[]);z=gzip.compress(json.dumps(bundle).encode(),mtime=0);(C/'reference-input-bundle.json.gz').write_bytes(z);print(len(cases),'manifest',sha(raw),'gzip',sha(z))
