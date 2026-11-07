# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json,xml.etree.ElementTree as ET
from pathlib import Path
C=Path(__file__).parent
sha=lambda b:hashlib.sha256(b).hexdigest()
z=(C/'size-reference-results.json.gz').read_bytes()
assert sha(z)=='2a38010f72b553b3f45779b0a4b6949e8fd588e66a20aa883832500636da0483'
assert gzip.decompress(z)==(C/'size-reference-results.json').read_bytes()
t=(C/'size-reference-transport.json').read_bytes()
assert sha(t)=='68f854cd60b6407a2d71b525baa687524236702edae5468eaf54f2f80b24b275'
assert base64.b64decode(json.loads(t)['data'])==z
v=json.loads(gzip.decompress(z));m=json.loads((C/'size-reference-manifest.json').read_text());e=v['environment']
assert e['manifest_sha256']==sha((C/'size-reference-manifest.json').read_bytes())
assert e['headless'] and e['jar_sha256']=='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053'
assert len(m['cases'])==len(v['results'])==8
rows=[];classes=[]
for q,r in zip(m['cases'],v['results']):
 s=Path(q['source']);b=s.read_bytes();assert q['name']==r['name'];assert sha(b)==q['source_sha256']==r['source_sha256']==r['stdin_sha256'];assert len(b)==r['source_bytes']==r['stdin_bytes']
 assert r['cwd']==e['worker']+'/new';assert r['actual_wrapper_argv']==[r['cwd'] if x=='PRIVATE_CWD' else x for x in q['invocation']]
 assert r['actual_java_argv']==[e['java'],'-Djava.awt.headless=true','-Dfile.encoding=UTF-8','-Duser.language=en','-Duser.country=AU','-Duser.timezone=GMT+11:00','-jar',e['jar'],'-pipe','-tsvg']
 assert not r['timed_out'] and r['timezone']=='GMT+11:00' and e['env_overrides']=={}
 for key,suffix in [('stdout','.svg'),('stderr','.stderr')]:
  data=base64.b64decode(r[key+'_base64'],validate=True);assert sha(data)==r[key+'_sha256'];s.with_suffix(suffix).write_bytes(data)
 root=ET.parse(s.with_suffix('.svg')).getroot();texts=[''.join(x.itertext()) for x in root.iter('{http://www.w3.org/2000/svg}text')]
 line=next(x for x in b.decode().splitlines() if x.startswith('N -> S: '));label=line.split(': ',1)[1];units=len(line.encode('utf-16-le'))//2
 assert texts.count(label)==1 and not s.with_suffix('.stderr').read_bytes() and r['exit_code']==0
 family=root.attrib['data-diagram-type'];assert family==('SEQUENCE' if units==2000 else 'CLASS')
 classes.append(dict(name=q['name'],source=str(s),source_sha256=sha(b),intended_valid=q['intended_valid'],observed_valid=True,java_exit=r['exit_code'],xml_valid=True,diagram_type=family,utf16_command_units=units,full_label_present=True,label_count=1,text_count=len(texts),stdout_sha256=r['stdout_sha256'],stderr_sha256=r['stderr_sha256'],finding='Sequence factory accepts' if units==2000 else 'Sequence command rejects; PSystemBuilder falls through to ClassDiagramFactory / CommandLinkClass, complete label preserved'))
 rows.append(dict(source=str(s),golden=str(s.with_suffix('.svg')),cohort='size8',axes=q['axes']))
(C/'size-reference-classification.json').write_text(json.dumps(classes,indent=2)+'\n');(C/'size8.json').write_text(json.dumps(rows,indent=2)+'\n');(C/'all3569.json').write_text(json.dumps(json.loads((C/'all3561.json').read_text())+rows,indent=2)+'\n')
print([(r['name'],r['diagram_type'],r['full_label_present']) for r in classes])
