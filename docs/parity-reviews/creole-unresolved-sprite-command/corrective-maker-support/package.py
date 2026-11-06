# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,tarfile,shutil
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();D=R/'docs/parity-reviews/creole-unresolved-sprite-command/corrective-maker-support';sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
# Every source, original membership, negative, stderr and comparison is retained.
# Binaries are rebuildable from the exact source/build hash records.
exclude={'target-before','target-after','before-rustuml','after-rustuml','probe','source-probe','standing-slice','__pycache__'}
files=[p for p in sorted(S.rglob('*')) if p.is_file() and not any(x in exclude for x in p.relative_to(S).parts)]
manifest=[dict(path=str(p.relative_to(S)),sha256=sha(p.read_bytes()),bytes=p.stat().st_size) for p in files]
dump(D/'archive-files.json',manifest)
archive=D/'measurements.tar.gz'
with tarfile.open(archive,'w:gz',compresslevel=6) as tar:
 for p in files:tar.add(p,arcname=str(p.relative_to(S)),recursive=False)
with tarfile.open(archive,'r:gz') as tar:
 members={m.name:m for m in tar.getmembers()};assert len(members)==len(manifest)
 for row in manifest:
  b=tar.extractfile(members[row['path']]).read();assert sha(b)==row['sha256'] and len(b)==row['bytes']
dump(D/'archive-integrity.json',dict(archive=archive.name,sha256=sha(archive.read_bytes()),bytes=archive.stat().st_size,files=len(manifest),all_member_bytes_verified=True,excluded_rebuildable=sorted(exclude)))
for name in ['prepare.py','run_replay.py','replay.py','validate_fresh.py','observations.py','summarize.py','standing_mapping.py','gates.py','provenance.py','evidence.py','package.py','unit-controls.py','summary.json','standing-summary.json','standing-mapping.json','model-observations.json','all-outcomes.json','fresh-classification.json','reference-validation.json','parent-upload-validation.json','parent-relay-validation.json','reference-transport.json','prepare-provenance.json','build-provenance.json','java-source-provenance.json','focused-commands.json','historical-integrity.json','dependency-handoff.json','causal-coverage.json','gates.json','unit-residue.json','request.json','manifest.json']:
 shutil.copy2(S/name,D/name)
print('verified archive',len(files),'files',archive.stat().st_size,'bytes',sha(archive.read_bytes()))
