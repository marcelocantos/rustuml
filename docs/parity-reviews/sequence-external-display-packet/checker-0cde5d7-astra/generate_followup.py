# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json
from pathlib import Path
C=Path(__file__).resolve().parent;D=C/'followup-inputs';D.mkdir(exist_ok=False);rows=[]
for kind in ['bare','created']:
 for engine in ['legacy','teoz']:
  name='ind5f_hide_widename_'+kind+'_'+engine
  body='hide stereotype\nparticipant Sender\n'+('create ' if kind=='created' else '')+'queue "Long Storage Inbox" as Inbox <<x>>\nSender -> Inbox: deliver\n'
  s='@startuml\n'+('!pragma teoz true\n' if engine=='teoz' else '')+body+'@enduml\n';p=D/(name+'.puml');p.write_text(s)
  rows.append(dict(name=name,source=str(p),source_sha256=hashlib.sha256(p.read_bytes()).hexdigest(),cwd='PRIVATE_CWD',timezone='GMT+11:00',invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=['hide stereotype','name wider than stereotype',kind,engine],intended_valid=True,mechanism='sequence-queue-complete-stereotype-display'))
m=dict(schema_version=1,purpose='Follow-up to isolate new hidden stereotype paint regression without old width error',cases=rows);data=(json.dumps(m,indent=2)+'\n').encode();(C/'followup-manifest.json').write_bytes(data)
b=dict(manifest=m,manifest_sha256=hashlib.sha256(data).hexdigest(),files=[dict(path=r['name']+'.puml',sha256=r['source_sha256'],base64=base64.b64encode(Path(r['source']).read_bytes()).decode()) for r in rows],resources=[]);(C/'followup-input-bundle.json.gz').write_bytes(gzip.compress((json.dumps(b,indent=2)+'\n').encode(),mtime=0))
for p in [C/'followup-manifest.json',C/'followup-input-bundle.json.gz']:print(p,hashlib.sha256(p.read_bytes()).hexdigest())
