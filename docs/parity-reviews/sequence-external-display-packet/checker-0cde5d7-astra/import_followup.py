# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json,xml.etree.ElementTree as ET
from pathlib import Path
C=Path(__file__).resolve().parent
sha=lambda b:hashlib.sha256(b).hexdigest()
for name,h in [('followup-reference-results.json.gz','6ff3fbe1961bf62653458a0aa9f9bc9c4d7d0c4011d7b6f90339df4f3e7fbdff'),('followup-reference-transport.json','93b23cc431413fba5b53427128500668dfba5ad0bca180747b22aca4490e3f50')]:assert sha((C/name).read_bytes())==h
assert gzip.decompress((C/'followup-reference-results.json.gz').read_bytes())==(C/'followup-reference-results.json').read_bytes()
v=json.loads((C/'followup-reference-results.json').read_text());m=json.loads((C/'followup-manifest.json').read_text());env=v['environment'];assert env['manifest_sha256']==sha((C/'followup-manifest.json').read_bytes());assert env['jar_sha256']=='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053';assert env['headless'] and env['locale']=='en_AU' and env['encoding']=='UTF-8' and env['env_overrides']=={}
assert len(m['cases'])==len(v['results'])==4
rows=[];manifest=[]
for req,r in zip(m['cases'],v['results']):
 assert req['name']==r['name'];assert req['source_sha256']==r['stdin_sha256']==r['source_sha256']==sha(Path(req['source']).read_bytes())
 assert r['cwd']==env['worker']+'/new' and r['timezone']=='GMT+11:00' and r['env_overrides']=={} and not r['timed_out']
 assert r['argv']==r['actual_wrapper_argv']==[r['cwd'] if x=='PRIVATE_CWD' else x for x in req['invocation']]
 assert r['java_argv']==r['actual_java_argv']==[env['java'],'-Djava.awt.headless=true','-Dfile.encoding=UTF-8','-Duser.language=en','-Duser.country=AU','-Duser.timezone=GMT+11:00','-jar',env['jar'],'-pipe','-tsvg']
 streams={k:base64.b64decode(r[k+'_base64'],validate=True) for k in ['stdout','stderr']}
 for k,b in streams.items():assert sha(b)==r[k+'_sha256']
 p=Path(req['source']);p.with_suffix('.svg').write_bytes(streams['stdout']);p.with_suffix('.stderr').write_bytes(streams['stderr'])
 root=ET.fromstring(streams['stdout']);texts=[''.join(e.itertext()) for e in root.iter() if e.tag.endswith('}text')];error=[s for s in texts if 'Nowhere to return' in s or 'Syntax Error' in s or 'Error'==s]
 valid=root.get('data-diagram-type')=='SEQUENCE' and not error and not streams['stderr']
 row=dict(req,observed_valid=valid,java_exit=r['exit_code'],root_attributes=root.attrib,error_labels=error,stderr=streams['stderr'].decode(),reference_sha256=r['stdout_sha256']);rows.append(row)
 manifest.append(dict(source=str(p),golden=str(p.with_suffix('.svg')),cohort='followup4',mechanism=req['mechanism'],axes=req['axes'],name=req['name'],intended_valid=req['intended_valid'],observed_valid=valid))
 if not valid:print(req['name'],root.attrib,error,streams['stderr'].decode())
(C/'followup-reference-validation.json').write_text(json.dumps(dict(cases=4,valid=sum(x['observed_valid'] for x in rows),invalid=sum(not x['observed_valid'] for x in rows),environment=env,records=rows),indent=2)+'\n');(C/'followup4.json').write_text(json.dumps(manifest,indent=2)+'\n')
