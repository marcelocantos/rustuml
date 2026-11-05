# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json
from pathlib import Path
C=Path(__file__).parent/'independent'/'unicode';C.mkdir(exist_ok=False);D=C/'reference-inputs';D.mkdir();sha=lambda b:hashlib.sha256(b).hexdigest();cases=[]
for name,space,valid in [('em_space','\u2003',False),('narrow_nbsp','\u202f',False),('nbsp','\u00a0',True)]:
 for engine in ['legacy','teoz']:
  n='indf535_terminator_'+name+'_'+engine;p=D/(n+'.puml');p.write_text('@startuml\n'+('!pragma teoz true\n' if engine=='teoz' else '')+'participant Tamarisk\nrnote over Tamarisk\npollen\nend'+space+'rnote\nTamarisk -> Tamarisk: after\n@enduml\n')
  cases.append(dict(name=n,mechanism='sequence-note-shape-terminator',source=str(p),source_sha256=sha(p.read_bytes()),cwd='PRIVATE_CWD',timezone='GMT+11:00',invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=['Pattern2 %s Java ASCII whitespace plus NBSP versus Rust Unicode whitespace',name,engine],intended_valid=valid))
m=dict(schema_version=1,purpose='Source-guided adversarial Unicode terminator whitespace supplement',cases=cases);raw=(json.dumps(m,indent=2)+'\n').encode();(C/'reference-manifest.json').write_bytes(raw);b=dict(manifest=m,manifest_sha256=sha(raw),files=[dict(path=p.name,sha256=sha(p.read_bytes()),base64=base64.b64encode(p.read_bytes()).decode()) for p in sorted(D.glob('*.puml'))],resources=[]);z=gzip.compress(json.dumps(b).encode(),mtime=0);(C/'reference-input-bundle.json.gz').write_bytes(z)
for p in [C/'reference-manifest.json',C/'reference-input-bundle.json.gz',*D.iterdir()]:p.chmod(0o444)
print(sha(raw),sha(z))
