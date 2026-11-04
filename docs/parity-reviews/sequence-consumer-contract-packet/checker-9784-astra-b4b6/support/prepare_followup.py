# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json
from pathlib import Path
C=Path(__file__).parent/'followup';C.mkdir();P=C/'reference-inputs';P.mkdir();sha=lambda b:hashlib.sha256(b).hexdigest();cases=[]
forms={'of_spaced':'note over of : reserved identity\n','of_tight':'note over of: reserved identity\n','of_coloured':'note over of #pink: reserved identity\n','of_multi_coloured':'note over of #pink\nreserved identity\nend note\n','of_multi':'note over of\nreserved identity\nend note\n','of_rnote':'rnote over of : reserved identity\n','of_hnote':'hnote over of : reserved identity\n','of_pair':'note over of, Kestrel: reserved identity\n'}
for key,note in forms.items():
 for engine in ['legacy','teoz']:
  name='b4b6_domain_'+key+'_'+engine
  source='@startuml\n'+('!pragma teoz true\n' if engine=='teoz' else '')+'participant "Reserved Alias" as of\nparticipant Kestrel\nof -> Kestrel: seed\n'+note+'@enduml\n'
  path=P/(name+'.puml');path.write_bytes(source.encode());cases.append(dict(name=name,source=str(path),source_sha256=sha(path.read_bytes()),cwd='PRIVATE_CWD',timezone='GMT+11:00',invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=['alias is optional grammar keyword of',key,engine],mechanism='sequence-note-over-command-domain',intended_valid=True))
m=dict(schema_version=1,purpose='Source-guided optional-of backtracking counterexample',cases=cases);raw=(json.dumps(m,indent=2)+'\n').encode();(C/'reference-manifest.json').write_bytes(raw);bundle=dict(manifest=m,manifest_sha256=sha(raw),files=[dict(path=p.name,sha256=sha(p.read_bytes()),base64=base64.b64encode(p.read_bytes()).decode()) for p in sorted(P.glob('*.puml'))],resources=[]);z=gzip.compress(json.dumps(bundle).encode(),mtime=0);(C/'reference-input-bundle.json.gz').write_bytes(z);print(len(cases),sha(raw),sha(z))
