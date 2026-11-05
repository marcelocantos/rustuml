# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,gzip,base64,hashlib
from pathlib import Path
C=Path(__file__).parent;D=C/'self-followup';D.mkdir();sha=lambda b:hashlib.sha256(b).hexdigest();cases=[]
for name,arrow in [('reverse_full','<-'),('reverse_open_top','//-'),('reverse_open_bottom','\\\\-'),('reverse_circle_top','o/-o'),('reverse_circle_bottom','o\\-o')]:
 for engine in ['legacy','teoz']:
  n='ind78_self_extra_'+name+'_'+engine;p=D/(n+'.puml');p.write_text('@startuml\n'+('!pragma teoz true\n' if engine=='teoz' else '')+'participant Vela\nparticipant Tucana\nparticipant Pavo\nactivate Vela\nactivate Vela\nVela '+arrow+' Vela: inward\ndeactivate Vela\ndeactivate Vela\nPavo '+arrow+' Pavo: outward\n@enduml\n')
  cases.append(dict(name=n,mechanism='sequence-self-half-polygon-order',source=str(p),source_sha256=sha(p.read_bytes()),cwd='PRIVATE_CWD',timezone='GMT+11:00',invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=['reverse '+name,'reversed declaration order','first/last nested activation',engine],intended_valid=True))
m=dict(schema_version=1,purpose='Complete independent reverse open/circle/full self consumer matrix',cases=cases);b=json.dumps(m,indent=2).encode()+b'\n';(C/'followup-reference-manifest.json').write_bytes(b)
r=dict(manifest=m,manifest_sha256=sha(b),files=[dict(path=x['name']+'.puml',sha256=x['source_sha256'],base64=base64.b64encode(Path(x['source']).read_bytes()).decode()) for x in cases],resources=[]);g=gzip.compress(json.dumps(r).encode(),mtime=0);(C/'followup-reference-input-bundle.json.gz').write_bytes(g);print(sha(b),sha(g))
