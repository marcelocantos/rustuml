# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Verify durable bytes without Java, networking, rendering or source mutation."""
import gzip,hashlib,json,tarfile
from pathlib import Path
D=Path(__file__).resolve().parent
R=next(p for p in D.parents if (p/'Cargo.toml').exists())
sha=lambda b:hashlib.sha256(b).hexdigest()
read=lambda p:json.loads(p.read_text())
artifacts=read(D/'artifact-integrity.json')
for entry in artifacts['files']:
 p=D/entry['path'];assert p.stat().st_size==entry['bytes'];assert sha(p.read_bytes())==entry['sha256'],str(p)
rows=read(D/'all-inputs.json')
measure=json.loads(gzip.decompress((D/'after.json.gz').read_bytes()))
assert len(rows)==len(measure)==3569
for row,m in zip(rows,measure):
 assert row['source']==m['source'] and row['golden']==m['golden']
 for key in ['source','golden']:assert sha((R/row[key]).read_bytes())==m[key+'_sha256'],row[key]
expected={r['path']:r for r in read(D/'capture-inventory.json')['files']}
with tarfile.open(D/'captures.tar.gz','r|gz') as archive:
 count=0
 for member in archive:
  assert member.isfile() and member.name in expected,member.name
  entry=expected.pop(member.name);h=hashlib.sha256();size=0
  with archive.extractfile(member) as f:
   while b:=f.read(1024*1024):h.update(b);size+=len(b)
  assert size==entry['bytes'] and h.hexdigest()==entry['sha256'],member.name
  count+=1
assert not expected
for prefix,want in [('','0240e3dbdb0ff6139c1afb08885c9e3235ce4e94bd02003ab15350279ec6abb9'),('probe-','ce1cfbfba5d8bb77827ba043233f0324b552ab8249d2824ccb3992827f1cd476'),('size-','2a38010f72b553b3f45779b0a4b6949e8fd588e66a20aa883832500636da0483')]:
 z=(D/(prefix+'reference-results.json.gz')).read_bytes();assert sha(z)==want;assert gzip.decompress(z)==(D/(prefix+'reference-results.json')).read_bytes()
print(json.dumps(dict(artifact_files=len(artifacts['files']),capture_files=count,paired_sources=len(rows),relay_gzip_hashes_verified=3,result='byte integrity verified; not parity acceptance'),indent=2))
