# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-review-b074'); D=S/'supplement-sources';D.mkdir(exist_ok=True);rows=[]
def add(mechanism,name,body,axes):
 p=D/(name+'.puml');data=('@startuml\n'+body+'\n@enduml\n').encode();p.write_bytes(data);rows.append(dict(name=p.name,source=str(p),source_sha256=hashlib.sha256(data).hexdigest(),source_base64=base64.b64encode(data).decode(),timezone='GMT+11:00',resources=[],mechanism=mechanism,axes=axes))
for linked in [False,True]:
 body='skinparam svgLinkTarget _parent\nclass Mooring\nnote "Detached chart" as Memo\nurl of Memo is [[https://example.org/chart?x=2&y=5{Inspect chart}]]'
 if linked: body+='\nMemo .. Mooring'
 add('class-note-url-group-ownership',f'note_url_command_{linked}',body,['named note','URL command','linked/unlinked'])
for symbol in ['actor','boundary']:
 for sub in [False,True]:
  label='Tide<sub>low</sub>' if sub else 'Horizon\\n'
  add('class-simple-symbol-painted-maximum',f'{symbol}_chrome_{sub}',f'allowmixing\ntitle Beacon register\ncaption Harbor caption\nfooter Coast footer\n<style>\n{symbol} {{\nFontSize 19\n}}\n</style>\nclass Mooring\n{symbol} "{label}" as Beacon\nMooring --> Beacon',['inline baseline or blank tail','body/chrome/canvas','symbol'])
raw=(json.dumps({'batch':'renderer-review-b074-supplement6','inputs':rows},separators=(',',':'))+'\n').encode();packed=gzip.compress(raw,mtime=0)
(S/'supplement-inputs.json').write_bytes(raw);(S/'supplement-inputs.json.gz').write_bytes(packed);(S/'supplement-manifest.json').write_text(json.dumps([{k:v for k,v in r.items() if k!='source_base64'} for r in rows],indent=2)+'\n');(S/'supplement-transport.json').write_text(json.dumps({'sha256':hashlib.sha256(packed).hexdigest(),'data':base64.b64encode(packed).decode()})+'\n');print(len(rows),hashlib.sha256(packed).hexdigest())
