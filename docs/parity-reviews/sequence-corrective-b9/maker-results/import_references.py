# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,hashlib,json,shutil
from pathlib import Path
c=Path(__file__).parent;d=Path('docs/parity-reviews/sequence-corrective-b9/maker-results');(d/'inputs').mkdir(parents=True,exist_ok=True)
def sha(b):return hashlib.sha256(b).hexdigest()
assert sha((c/'reference-results.json.gz').read_bytes())=='084b60166689371184df41d99f39e7340efc8cb42c3363f3c12ed92e0f80a5ef'
assert sha((c/'reference-manifest.json').read_bytes())=='758da5f6cf52580a2ae83d3ef4f8cb33f01a6e4fba865f4585ff8228cb497912'
manifest=json.loads((c/'reference-manifest.json').read_text());capture=json.loads((c/'reference-results.json').read_text());results={Path(r['name']).stem:r for r in capture['results']};assert len(results)==len(manifest['cases'])==20
records=[]
for case in manifest['cases']:
 r=results[case['name']];src=Path(case['source']).read_bytes();assert sha(src)==case['source_sha256']==r['source_sha256'];assert not r['timed_out'];assert r['timezone']==case['timezone'];assert '-Djava.awt.headless=true' in r['java_argv'];assert r['argv']==[r['cwd'] if x==case['cwd'] else x for x in case['invocation']]
 out=base64.b64decode(r['stdout_base64'],validate=True);err=base64.b64decode(r['stderr_base64'],validate=True);assert sha(out)==r['stdout_sha256'] and sha(err)==r['stderr_sha256']
 p=d/'inputs'/(case['name']+'.puml');p.write_bytes(src);p.with_suffix('.svg').write_bytes(out);p.with_suffix('.stderr').write_bytes(err);p.with_suffix('.provenance.json').write_text(json.dumps(dict({k:v for k,v in r.items() if not k.endswith('_base64')}, reference_environment='../reference-environment.json', runtime_equivalence='OpenJDK21.0.5 and Graphviz12.2.0 differ from origin; pinned JAR and scoped seven-case relay conformance do not establish global equivalence.'),indent=2)+'\n')
 records.append(dict(source=str(p),golden=str(p.with_suffix('.svg')),cohort='new-maker20',axes=case['axes'],java_exit=r['exit_code']))
(d/'new-manifest.json').write_text(json.dumps(records,indent=2)+'\n');(d/'reference-environment.json').write_text(json.dumps(capture['environment'],indent=2)+'\n')
for old,new in [('reference-results.json.gz','reference-results.json.gz'),('reference-manifest.json','reference-request.json')]:shutil.copyfile(c/old,d/new)
p=Path('docs/parity-reviews/sequence-lifecycle-decoration-packet/checker-b9c6');fresh=json.loads((p/'fresh42.json').read_text())+json.loads((p/'fresh7.json').read_text());assert len(fresh)==49
(c/'focus69.json').write_text(json.dumps(fresh+records,indent=2)+'\n')
(c/'all1897.json').write_text(json.dumps(json.loads(Path('docs/parity-reviews/sequence-lifecycle-decoration-packet/maker-results/all-inputs.json').read_text())+fresh+records,indent=2)+'\n')
(c/'bin').mkdir(exist_ok=True)
for old,new in [('/Users/marcelo/Library/Caches/rustuml-sequence-repair/target/release/rustuml','rustuml'),('/Users/marcelo/Library/Caches/rustuml-sequence-repair/target/release/examples/diff_one','diff_one')]:shutil.copy2(old,c/'bin'/new)
shutil.copy2(c/'before-bin/compare_cli',c/'bin/compare_cli')
