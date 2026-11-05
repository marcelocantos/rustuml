# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json
from pathlib import Path
C=Path(__file__).parent/'independent'/'boundary';C.mkdir(exist_ok=False);D=C/'reference-inputs';D.mkdir();sha=lambda b:hashlib.sha256(b).hexdigest();cases=[]
# Java-compatible sans-serif line metrics yield 603/512 per font-size unit.
# Sum 512 gives 603 exactly; sum 557 gives 655 + 511/512, just below 656.
# These supplement, never replace, the earlier exploratory boundary probes.
for shape in ['note','rnote','hnote']:
 for label,sizes in [('exact512',[100,101,102,103,106]),('below557',[76,77,78,79,80,81,86])]:
  for engine in ['legacy','teoz']:
   n='indf535_'+shape+'_'+label+'_'+engine;p=D/(n+'.puml');p.write_text('@startuml\n'+('!pragma teoz true\n' if engine=='teoz' else '')+'participant Viburnum\n'+shape+' over Viburnum\n'+'\n'.join(f'<size:{size}>v</size>' for size in sizes)+'\nend note\nViburnum -> Viburnum: grow\n@enduml\n')
   cases.append(dict(name=n,mechanism='sequence-note-aggregate-paint-height',source=str(p),source_sha256=sha(p.read_bytes()),cwd='PRIVATE_CWD',timezone='GMT+11:00',invocation=['/Users/marcelo/rustuml-reference/bin/reference','--cwd','PRIVATE_CWD','--timezone','GMT+11:00','--','-pipe','-tsvg'],axes=[shape,label,str(len(sizes))+' mixed-size rows',engine],intended_valid=True,font_sizes=sizes,expected_text_height_numerator=sum(sizes)*603,expected_text_height_denominator=512))
m=dict(schema_version=1,purpose='Exact and just-below aggregate cast boundaries, following source font metric verification',cases=cases);raw=(json.dumps(m,indent=2)+'\n').encode();(C/'reference-manifest.json').write_bytes(raw);b=dict(manifest=m,manifest_sha256=sha(raw),files=[dict(path=p.name,sha256=sha(p.read_bytes()),base64=base64.b64encode(p.read_bytes()).decode()) for p in sorted(D.glob('*.puml'))],resources=[]);z=gzip.compress(json.dumps(b).encode(),mtime=0);(C/'reference-input-bundle.json.gz').write_bytes(z)
for p in [C/'reference-manifest.json',C/'reference-input-bundle.json.gz',*D.iterdir()]:p.chmod(0o444)
print(sha(raw),sha(z))
