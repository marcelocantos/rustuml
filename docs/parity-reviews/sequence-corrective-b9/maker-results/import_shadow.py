# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64
import hashlib
import json
from pathlib import Path
import shutil
c=Path(__file__).parent
d=Path('docs/parity-reviews/sequence-corrective-b9/maker-results')
def sha(b):return hashlib.sha256(b).hexdigest()
assert sha((c/'shadow-value-reference-results.json.gz').read_bytes())=='92f4fa57a19633895c66ce8135f3530625489db2714371c23ad5482f3fbca15d'
assert sha((c/'shadow-value-reference-manifest.json').read_bytes())=='f074dc38629881c9d540ccbd0432e14b42f1188c2608c1685d9164169bffc9ce'
m=json.loads((c/'shadow-value-reference-manifest.json').read_text());raw=json.loads((c/'shadow-value-reference-results.json').read_text());rs={Path(r['name']).stem:r for r in raw['results']};assert len(rs)==len(m['cases'])==4
assert raw['environment']['jar_sha256']=='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053'
records=[]
for case in m['cases']:
 r=rs[case['name']];src=Path(case['source']).read_bytes();assert sha(src)==case['source_sha256']==r['source_sha256'];assert not r['timed_out'];assert r['timezone']==case['timezone'];assert '-Djava.awt.headless=true' in r['java_argv'];assert r['argv']==[r['cwd'] if x==case['cwd'] else x for x in case['invocation']]
 out=base64.b64decode(r['stdout_base64'],validate=True);err=base64.b64decode(r['stderr_base64'],validate=True);assert sha(out)==r['stdout_sha256'] and sha(err)==r['stderr_sha256']
 p=d/'inputs'/(case['name']+'.puml');p.write_bytes(src);p.with_suffix('.svg').write_bytes(out);p.with_suffix('.stderr').write_bytes(err);p.with_suffix('.provenance.json').write_text(json.dumps(dict({k:v for k,v in r.items() if not k.endswith('_base64')}, reference_environment='../shadow-reference-environment.json', runtime_equivalence='OpenJDK21.0.5 and Graphviz12.2.0 differ from origin; pinned JAR and scoped seven-case relay conformance do not establish global equivalence.'),indent=2)+'\n')
 records.append(dict(source=str(p),golden=str(p.with_suffix('.svg')),cohort='shadow-value4',axes=case['axes'],java_exit=r['exit_code']))
(d/'shadow-manifest.json').write_text(json.dumps(records,indent=2)+'\n');(d/'shadow-reference-environment.json').write_text(json.dumps(raw['environment'],indent=2)+'\n')
shutil.copyfile(c/'shadow-value-reference-results.json.gz',d/'shadow-reference-results.json.gz');shutil.copyfile(c/'shadow-value-reference-manifest.json',d/'shadow-reference-request.json')
(c/'shadow4.json').write_text(json.dumps(records,indent=2)+'\n')
