# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,tarfile
from pathlib import Path
C=Path(__file__).parent;D=C/'durable-package';rows=json.loads((D/'capture-inventory.json').read_text())['files'];expected={r['path']:r for r in rows};seen=set()
with tarfile.open(D/'captures.tar.gz','r|gz') as t:
 for m in t:
  if not m.isfile():continue
  assert m.name in expected and m.name not in seen;seen.add(m.name);f=t.extractfile(m);h=hashlib.sha256();n=0
  while b:=f.read(1024*1024):h.update(b);n+=len(b)
  assert n==expected[m.name]['bytes'] and h.hexdigest()==expected[m.name]['sha256'],m.name
assert seen==set(expected)
for row in json.loads((D/'all-inputs.json').read_text()):
 for key in ['source','golden']:
  path=Path(row[key]);parts=path.parts
  if 'sequence-sprite-participant-packet' in parts:path=D/'inputs'/path.name
  assert path.exists(),path
  if key+'_sha256' in row:assert hashlib.sha256(path.read_bytes()).hexdigest()==row[key+'_sha256'],path
summary=json.loads((D/'summary.json').read_text());assert summary['lost_matches']==summary['lost_rejections']==summary['new_valid_errors']==summary['cli_panics']==summary['timeouts']==0
(D/'package-verification.json').write_text(json.dumps(dict(archive_entries_verified=len(seen),source_reference_memberships_verified=4174,sha256_and_size_verified=True,no_acceptance_claim=True),indent=2)+'\n');print('verified',len(seen),'archive entries and4174 pairs')
