import base64,gzip,hashlib,json,xml.etree.ElementTree as ET
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-class-cff6')
raw=(S/'reference-results.json.gz').read_bytes();assert hashlib.sha256(raw).hexdigest()=='afc25b5c5958bd8643b43ab180e5d6bca96be0ddb4d25562469787dab740e4b4'
x=json.loads(gzip.decompress(raw));assert x==json.loads((S/'reference-results.json').read_text())
manifest={r['name']:r for r in json.loads((S/'fresh-manifest.json').read_text())};assert len(x['results'])==len(manifest)==26
assert x['environment']['jar_sha256']=='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053'
checks=[]
for r in x['results']:
 m=manifest.pop(r['name']);p=Path(m['source']);assert hashlib.sha256(p.read_bytes()).hexdigest()==m['source_sha256']==r['source_sha256']
 for stream in ['stdout','stderr']:
  b=base64.b64decode(r[stream+'_base64']);assert hashlib.sha256(b).hexdigest()==r[stream+'_sha256'];p.with_suffix('.svg' if stream=='stdout' else '.java.stderr').write_bytes(b)
 assert not r['timed_out'] and r['exit_code']==0 and r['timezone']=='GMT+11:00'
 assert '-Djava.awt.headless=true' in r['java_argv']
 try:root=ET.fromstring(p.with_suffix('.svg').read_bytes());xml=root.tag
 except ET.ParseError as e:xml=str(e)
 checks.append({'name':r['name'],'source_sha256':r['source_sha256'],'xml':xml,'raw_exit':r['exit_code']})
assert not manifest
(S/'reference-validation.json').write_text(json.dumps(checks,indent=2)+'\n')
(S/'fresh26.json').write_text(json.dumps([str(p) for p in sorted((S/'fresh-sources').glob('*.puml'))]))
