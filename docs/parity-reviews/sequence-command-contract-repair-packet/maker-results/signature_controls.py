# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,gzip,base64,hashlib
from pathlib import Path
C=Path(__file__).parent;D=C/'signature-inputs';D.mkdir();sha=lambda b:hashlib.sha256(b).hexdigest();cases=[]
for case,command in [('outer_bit','create actor\vWillow'),('inner_bit','create\vactor Willow')]:
 for teoz in [False,True]:
  name='create_'+case+('_teoz' if teoz else '_legacy');p=D/(name+'.puml');p.write_text('@startuml\n'+('!pragma teoz true\n' if teoz else '')+'participant Birch\n'+command+'\nBirch -> Willow: born\nWillow -> Birch: ready\n@enduml\n');cases.append(dict(name=name,source=str(p),source_sha256=sha(p.read_bytes()),cwd='PRIVATE_CWD',timezone='GMT+11:00',env={},invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=['FoxSignature full command vs parsed declaration tail',case,'teoz' if teoz else 'legacy'],intended_valid=True))
m=dict(schema_version=1,purpose='Preserve full-command signature scope through create declaration parsing',cases=cases);raw=(json.dumps(m,indent=2)+'\n').encode();(C/'signature-reference-manifest.json').write_bytes(raw);b=dict(manifest=m,manifest_sha256=sha(raw),files=[dict(path=p.name,sha256=sha(p.read_bytes()),base64=base64.b64encode(p.read_bytes()).decode()) for p in sorted(D.glob('*.puml'))],resources=[]);z=gzip.compress(json.dumps(b).encode(),mtime=0);(C/'signature-reference-input-bundle.json.gz').write_bytes(z);print('manifest',sha(raw),'bundle',sha(z))
