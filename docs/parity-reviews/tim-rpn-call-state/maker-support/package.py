# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Preserve reviewed maker captures in a new evidence directory, never old records."""
import gzip,hashlib,json,shutil,tarfile
from pathlib import Path
R=Path.cwd();S=Path('/tmp/rustuml-tim6');D=R/'docs/parity-reviews/tim-rpn-call-state/maker-support';D.mkdir(parents=True,exist_ok=True)
for name in ['new-manifest.json','remote-request.json','integrity.json','analysis.json','negative-statuses.json','java-source-provenance.json','gates.json','controls.json','replay.py','relay_eval.py','analyze.py','integrity.py','controls.py','gates.py','prepare.py','package.py']:
 shutil.copy2(S/name,D/name)
for p in S.glob('*-summary.json'):shutil.copy2(p,D/p.name)
for p in S.glob('final-*.json'):
 if p.name.endswith('-summary.json'):continue
 (D/(p.name+'.gz')).write_bytes(gzip.compress(p.read_bytes(),mtime=0))
shutil.copytree(S/'new',D/'inputs',dirs_exist_ok=True)
shutil.copytree(S/'relay18-final',D/'relay18',dirs_exist_ok=True)
probe=R/'docs/parity-reviews/tim-corrective-execution-maker/SourceProbe.java';shutil.copy2(probe,D/'SourceProbe.java')
# New inputs retain same-stem exact Java SVGs, including empty/error references.
for p in (D/'inputs').glob('*.puml'):shutil.copy2(S/'relay18-final'/p.stem/'java.svg.stdout',p.with_suffix('.svg'))
# The archive restores the original scratch hierarchy; raw output bytes are never
# normalized. Include every intermediate failure, command record, model and SVG.
archive=D/'measurements.tar.gz'
with tarfile.open(archive,'w:gz',compresslevel=9) as t:
 for p in sorted(S.iterdir()):
  if p.name in ['before-rustuml','iteration1-rustuml','final-rustuml','probe']:continue
  t.add(p,arcname=p.name)
files=[]
for p in sorted(D.rglob('*')):
 if p.is_file() and p.name not in ['README.md','files.json']:
  b=p.read_bytes();files.append(dict(path=str(p.relative_to(D)),bytes=len(b),sha256=hashlib.sha256(b).hexdigest()))
(D/'files.json').write_text(json.dumps(files,indent=2)+'\n')
print('archive',archive.stat().st_size,hashlib.sha256(archive.read_bytes()).hexdigest(),'files',len(files))
