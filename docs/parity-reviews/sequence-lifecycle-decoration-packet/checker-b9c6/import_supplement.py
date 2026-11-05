# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json,shutil
from pathlib import Path
root=Path.cwd();c=Path('/Users/marcelo/Library/Caches/rustuml-sequence-review-b9c6cf04');relay=Path('/Users/marcelo/Library/Caches/rustuml-campaign-relay')
def sha(b):return hashlib.sha256(b).hexdigest()
r=json.loads((c/'supplement-request.json').read_text());data=(relay/'sequence7-results.json.gz').read_bytes();assert sha(data)=='482551b1dbac55b80a242e697981c24136709b36130084335c2cc44975faac13'
x=json.loads(gzip.decompress(data));env=x['environment']; assert env['jar_sha256']==r['jar_sha256']; assert env['headless'] is True;assert env['manifest_sha256']==sha((c/'supplement-request.json').read_bytes())
by={row['name']:row for row in x['results']};assert set(by)=={row['remote_relative_name'] for row in r['cases']}
manifest=[]
for case in r['cases']:
 row=by[case['remote_relative_name']];body=Path(case['source']).read_bytes();assert sha(body)==case['source_sha256']==row['source_sha256'];assert row['timed_out'] is False
 argv=[row['cwd'] if a=='PRIVATE_CWD' else a for a in case['invocation']];assert row['argv']==argv;assert row['timezone']==case['timezone'];assert '-Djava.awt.headless=true' in row['java_argv'];assert '-Duser.timezone=GMT+11:00' in row['java_argv']
 out=base64.b64decode(row['stdout_base64'],validate=True);err=base64.b64decode(row['stderr_base64'],validate=True);assert sha(out)==row['stdout_sha256'];assert sha(err)==row['stderr_sha256']
 d=root/'test-diagrams/perturbations'/case['mechanism'];d.mkdir(exist_ok=True)
 (d/(case['name']+'.puml')).write_bytes(body);(d/(case['name']+'.svg')).write_bytes(out);(d/(case['name']+'.java.stderr')).write_bytes(err)
 provenance={k:v for k,v in row.items() if 'base64' not in k};provenance['environment']=env;provenance['axes']=case['axes'];(d/(case['name']+'.provenance.json')).write_text(json.dumps(provenance,indent=2)+'\n')
 manifest.append(dict(source=str((d/(case['name']+'.puml')).relative_to(root)),golden=str((d/(case['name']+'.svg')).relative_to(root)),cohort=case['mechanism'],axes=case['axes'],java_exit=row['exit_code']))
(c/'fresh7.json').write_text(json.dumps(manifest,indent=2)+'\n')
e=root/'docs/parity-reviews/sequence-lifecycle-decoration-packet/checker-b9c6';e.mkdir(exist_ok=True)
for src,name in [(c/'supplement-request.json','supplement-request.json'),(relay/'sequence7-results.json.gz','supplement-results.json.gz'),(relay/'sequence7-transport.json','supplement-transport.json')]:shutil.copyfile(src,e/name)
(e/'supplement-validation.json').write_text(json.dumps(dict(cases=len(manifest),manifest_sha256=sha((c/'supplement-request.json').read_bytes()),results_gzip_sha256=sha(data),exact_membership_and_sources=True,streams_and_invocations_verified=True,environment=env),indent=2)+'\n')
print('Imported and verified',len(manifest))
