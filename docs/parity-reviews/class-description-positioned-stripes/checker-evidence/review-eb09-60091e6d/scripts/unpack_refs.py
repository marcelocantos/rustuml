# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json,sys
from pathlib import Path
S=Path(__file__).resolve().parent
manifest,result,expected=sys.argv[1:];mp=S/manifest;p=S/result
raw=p.read_bytes();gz=p.with_suffix(p.suffix+'.gz').read_bytes();assert hashlib.sha256(gz).hexdigest()==expected;assert gzip.decompress(gz)==raw
m=json.loads(mp.read_text());r=json.loads(raw);by={c['name']:c for c in m['cases']};assert set(by)=={c['name'] for c in r['results']};env=r['environment'];assert env['jar_sha256']==m['jar_sha256']
rows=[]
for c in r['results']:
 orig=by[c['name']];src=Path(orig['source']);assert src.read_bytes()==base64.b64decode(orig['source_base64']);assert hashlib.sha256(src.read_bytes()).hexdigest()==orig['source_sha256']==c['source_sha256']
 out=base64.b64decode(c['stdout_base64']);err=base64.b64decode(c['stderr_base64']);assert hashlib.sha256(out).hexdigest()==c['stdout_sha256'];assert hashlib.sha256(err).hexdigest()==c['stderr_sha256'];src.with_suffix('.svg').write_bytes(out);src.with_suffix('.java.stderr').write_bytes(err)
 record={k:v for k,v in c.items() if k not in ['stdout_base64','stderr_base64']};src.with_suffix('.java.json').write_text(json.dumps(record,indent=2)+'\n');rows.append(record)
(S/(p.stem+'-validation.json')).write_text(json.dumps(dict(manifest=manifest,manifest_sha256=hashlib.sha256(mp.read_bytes()).hexdigest(),result=result,gzip_sha256=expected,environment=env,results=rows),indent=2)+'\n')
print('verified and unpacked',len(rows),'keys',list(rows[0]))
