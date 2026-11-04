# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json
from pathlib import Path
s=Path(__file__).resolve().parent;out=s/'fresh-sources';out.mkdir(exist_ok=True);rows=[]
def add(name,body,axes):
 p=out/(name+'.puml');data=('@startuml\n'+body+'\n@enduml\n').encode();p.write_bytes(data);rows.append(dict(name=p.name,source=str(p),source_sha256=hashlib.sha256(data).hexdigest(),source_base64=base64.b64encode(data).decode(),timezone='GMT+11:00',axes=axes))
for size in [10,22]:
 for lr in [False,True]:
  for multi in [False,True]:
   label='Relay desk\\nQueue records' if multi else 'Archive vestibule'
   style=f'<style>\nboundary {{\nFontSize {size}\nLineColor #425566\nLineThickness 2\nBackgroundColor #DDEEFF\n}}\n</style>'
   direction='left to right direction' if lr else 'top to bottom direction'
   stereo=' <<perimeter>>' if multi else ''
   add(f'boundary_{size}_{lr}_{multi}',f'allowmixing\n{direction}\n{style}\nclass Anchor\nboundary "{label}" as Gate{stereo}\nAnchor --> Gate',['font size','label/stereotype','rank direction','line/fill style'])
for size in [10,22]:
 for shadow in [False,True]:
  add(f'actor_{size}_{shadow}',f'allowmixing\nleft to right direction\nskinparam shadowing {str(shadow).lower()}\n<style>\nactor {{\nFontSize {size}\nLineThickness 2\n}}\n</style>\nclass Cabinet\nactor "Duty officer\\nRegistration" as Clerk <<custodian>>\nClerk --> Cabinet',['font size','shadow','multiline label','stroke'])
for i,position in enumerate(['top','right','bottom','left']):
 for braced in [False,True]:
  group=(i+braced)%2==0
  entities='package "Archive room" as Room {\nclass Ledger\n}' if group else 'class Ledger'
  target='Room' if group else 'Ledger'
  opening=' {' if braced else '';closing='}' if braced else 'end note'
  add(f'note_{position}_{braced}',f'skinparam svgLinkTarget _blank\n{entities}\nnote {position} of {target} $audit <<memo>> #LightYellow [[https://example.org/records?view=recent&sort=id{{Open records}}]]{opening}\ncomponent is prose here\nclass remains prose too\n{closing}',['position','brace/end-note','leaf/group target','URL tooltip and target'])
raw=(json.dumps({'batch':'class20-simple-note','inputs':rows},separators=(',',':'))+'\n').encode();packed=gzip.compress(raw,mtime=0);(s/'fresh-inputs.json.gz').write_bytes(packed);(s/'fresh-manifest.json').write_text(json.dumps([{k:v for k,v in r.items() if k!='source_base64'} for r in rows],indent=2)+'\n');(s/'fresh-transport.json').write_text(json.dumps({'sha256':hashlib.sha256(packed).hexdigest(),'data':base64.b64encode(packed).decode()},separators=(',',':'))+'\n');print('inputs',len(rows),'compressed_sha256',hashlib.sha256(packed).hexdigest())
