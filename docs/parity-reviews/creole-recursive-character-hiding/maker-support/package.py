# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,tarfile,shutil,os
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();D=R/'docs/parity-reviews/creole-recursive-character-hiding/maker-support';sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
exclude={'target-before','target-after','before-rustuml','after-rustuml','after-initial-rustuml','probe','__pycache__'}
files=[]
for root,dirs,names in os.walk(S):
 dirs[:]=sorted(d for d in dirs if d not in exclude)
 files.extend(Path(root)/n for n in sorted(names) if n not in exclude)
files.sort();manifest=[dict(path=str(p.relative_to(S)),sha256=sha(p.read_bytes()),bytes=p.stat().st_size) for p in files]
dump(D/'archive-files.json',manifest)
archive=D/'measurements.tar.gz'
with tarfile.open(archive,'w:gz',compresslevel=6) as tar:
 for p in files:tar.add(p,arcname=str(p.relative_to(S)),recursive=False)
with tarfile.open(archive,'r:gz') as tar:
 members={m.name:m for m in tar.getmembers()};assert len(members)==len(manifest)
 for row in manifest:
  b=tar.extractfile(members[row['path']]).read();assert sha(b)==row['sha256'] and len(b)==row['bytes']
dump(D/'archive-integrity.json',dict(archive=archive.name,sha256=sha(archive.read_bytes()),bytes=archive.stat().st_size,files=len(manifest),all_member_bytes_verified=True,excluded_rebuildable=sorted(exclude)))
for name in ['summary.json','loss-error-summary.json','gate-summary.json','gates.json','historical-integrity.json','standing-summary.json','model-observations.json','model-effects-summary.json','dependency-handoff.json','causal-coverage.json','build-provenance.json','java-source-provenance.json','focused-results.json','relay-envelope-hashes.json','reference-validation.json','fresh-classification.json','registered-pairs.json','unit-residue.json','prepare.py','run_replay.py','replay.py','summarize.py','models.py','observations.py','standing_mapping.py','gates.py','provenance.py','evidence.py','package.py','validate_fresh.py']:
 shutil.copy2(S/name,D/name)
print('verified archive',len(files),'files',archive.stat().st_size,'bytes',sha(archive.read_bytes()),flush=True)
