import json,gzip,hashlib,base64,xml.etree.ElementTree as ET,shutil,subprocess
from pathlib import Path
C=Path(__file__).parent;h=lambda b:hashlib.sha256(b).hexdigest();z=(C/'reference-results.json.gz').read_bytes();assert h(z)=='fe3e044a7f09e30bb04b2cd19c4879e924cc0d9d82e205f2a5ebe59fad465f54';raw=(C/'reference-results.json').read_bytes();assert gzip.decompress(z)==raw;v=json.loads(raw);m=json.loads((C/'reference-manifest.json').read_text());assert len(v['results'])==len(m['cases'])==20
rows=[];classifications=[]
for req,r in zip(m['cases'],v['results']):
 assert req['name']==r['name'];s=Path(req['source']);body=s.read_bytes();assert h(body)==req['source_sha256']==r['source_sha256']==r['stdin_sha256'];assert len(body)==r['source_bytes']==r['stdin_bytes'];assert r['actual_wrapper_argv']==[r['cwd'] if a=='PRIVATE_CWD' else a for a in req['invocation']];assert r['timezone']=='GMT+11:00' and not r['timed_out'];assert '-Djava.awt.headless=true' in r['actual_java_argv'];assert '-Duser.timezone=GMT+11:00' in r['actual_java_argv']
 for stream in ['stdout','stderr']:
  b=base64.b64decode(r[stream+'_base64'],validate=True);assert h(b)==r[stream+'_sha256'];s.with_suffix('.svg' if stream=='stdout' else '.stderr').write_bytes(b)
 root=ET.fromstring(s.with_suffix('.svg').read_bytes());text=' '.join(root.itertext());valid=r['exit_code']==0 and root.attrib.get('data-diagram-type')=='SEQUENCE' and 'Syntax Error' not in text
 classifications.append(dict(name=r['name'],java_exit=r['exit_code'],xml_valid=True,diagram_type=root.attrib.get('data-diagram-type'),syntax_error='Syntax Error' in text,intended_valid=req['intended_valid'],observed_valid=valid,source_sha256=h(body),stdout_sha256=r['stdout_sha256'],stderr_sha256=r['stderr_sha256']))
 rows.append(dict(source=str(s),golden=str(s.with_suffix('.svg')),cohort='maker20',axes=req['axes']))
(C/'reference-classification.json').write_text(json.dumps(classifications,indent=2)+'\n');(C/'maker20.json').write_text(json.dumps(rows,indent=2)+'\n');(C/'all2169.json').write_text(json.dumps(json.loads((C/'all2147.json').read_text())+rows+json.loads((C/'handoff2.json').read_text()),indent=2)+'\n');(C/'focus86.json').write_text(json.dumps(json.loads((C/'checker64.json').read_text())+rows+json.loads((C/'handoff2.json').read_text()),indent=2)+'\n')
b=C/'first-bin';b.mkdir(exist_ok=True);t=Path('/Users/marcelo/Library/Caches/rustuml-sequence-repair/target/release')
for name,p in [('rustuml',t/'rustuml'),('diff_one',t/'examples/diff_one'),('compare_cli',C/'before-bin/compare_cli')]:shutil.copy2(p,b/name)
files=['crates/rustuml-parser/src/diagram/sequence.rs','crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs'];snap=C/'first-source';snap.mkdir(exist_ok=True)
for p in files:shutil.copy2(p,snap/(p.split('/')[1]+'-'+p.split('/')[-2]+'.rs'))
(C/'first-binding.json').write_text(json.dumps(dict(head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),dirty=True,binaries={p.name:h(p.read_bytes()) for p in b.iterdir()},source_files={p:h(Path(p).read_bytes()) for p in files}),indent=2)+'\n');print('valid',sum(r['observed_valid'] for r in classifications),'invalid',sum(not r['observed_valid'] for r in classifications))
