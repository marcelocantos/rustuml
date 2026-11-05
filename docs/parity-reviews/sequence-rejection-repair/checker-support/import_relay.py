# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Verify the returned T3 bundle and preserve raw oracle streams, without Java."""
import base64
import gzip
import hashlib
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
OUT=Path(__file__).resolve().parent
RELAY=Path('/Users/marcelo/Library/Caches/rustuml-campaign-relay')
def sha(data): return hashlib.sha256(data).hexdigest()
compressed=(RELAY/'sequence68-results.json.gz').read_bytes()
assert sha(compressed)=='ac20ceb484bce43d5dbbef57e30914022b916a318282ac596884d14ef5e8dfe7'
raw=gzip.decompress(compressed)
assert raw==(RELAY/'sequence68-results.json').read_bytes()
bundle=json.loads(raw)
manifest=json.loads((OUT/'fresh-manifest.json').read_text())
expected={r['source']:r for r in manifest}
assert len(expected)==len(bundle['results'])==68
assert set(expected)=={r['source'] for r in bundle['results']}
env=bundle['environment']
assert env['jar_sha256']=='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053'
assert env['timezone']=='GMT+11:00'
rows=[]
# Validate every entry before writing any reference.
for r in bundle['results']:
 p=ROOT/r['source'];assert sha(p.read_bytes())==r['source_sha256']
 assert r['argv'][3:]==['--timezone','GMT+11:00','--','-pipe','-tsvg']
 assert not r['timed_out']
 stdout=base64.b64decode(r['stdout_base64'],validate=True)
 stderr=base64.b64decode(r['stderr_base64'],validate=True)
 assert sha(stdout)==r['stdout_sha256'] and sha(stderr)==r['stderr_sha256']
 rows.append((p,r,stdout,stderr))
for p,r,stdout,stderr in rows:
 p.with_suffix('.svg').write_bytes(stdout)
 p.with_suffix('.stderr').write_bytes(stderr)
 p.with_suffix('.provenance.json').write_text(json.dumps({k:v for k,v in r.items() if not k.endswith('_base64')},indent=2)+'\n')
(OUT/'remote-captures.json.gz').write_bytes(compressed)
(OUT/'remote-environment.json').write_text(json.dumps(env,indent=2)+'\n')
(OUT/'transport.json').write_bytes((RELAY/'sequence68-transport.json').read_bytes())
(OUT/'reference-import.json').write_text(json.dumps(dict(count=68,compressed_sha256=sha(compressed),uncompressed_sha256=sha(raw),input_compressed_sha256=env['input_compressed_sha256'],transport='Temporary T3 relay via existing parent and remote runner threads; no local Java or SSH',command='python3 docs/parity-reviews/sequence-rejection-repair/checker-support/import_relay.py',source_hashes={r['source']:r['source_sha256'] for _,r,_,_ in rows}),indent=2)+'\n')
print('Verified and preserved all 68 raw references and stderr streams.')
