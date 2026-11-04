# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,tarfile,shutil
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();D=R/'docs/parity-reviews/creole-recursive-character-hiding/checker-support';D.mkdir(parents=True,exist_ok=True);sha=lambda b:hashlib.sha256(b).hexdigest()
paths=[p for p in S.iterdir() if p.is_file() and p.suffix in ['.json','.py','.md','.gz'] and p.name not in ['measurements.tar.gz']]
paths.append(S/'probe.rs')
for name in ['replay','new','fresh-reference','cwd-resources','unit-residue','logs','maker/frozen','maker/new','maker/fresh-reference']:
 paths.extend(p for p in (S/name).rglob('*') if p.is_file() and '__pycache__' not in p.parts)
paths=sorted(set(paths));inventory=[dict(path=str(p.relative_to(S)),sha256=sha(p.read_bytes()),bytes=p.stat().st_size) for p in paths]
with (D/'measurements.tar.gz').open('wb') as raw:
 with gzip.GzipFile(fileobj=raw,mode='wb',mtime=0) as gz:
  with tarfile.open(fileobj=gz,mode='w') as tar:
   for p in paths:tar.add(p,arcname=str(p.relative_to(S)),recursive=False)
archive=D/'measurements.tar.gz'
readback=[]
with tarfile.open(archive) as tar:
 for m in tar:
  b=tar.extractfile(m).read();readback.append(dict(path=m.name,sha256=sha(b),bytes=len(b)))
assert readback==inventory
(D/'archive-files.json').write_text(json.dumps(inventory,indent=2)+'\n');(D/'archive-integrity.json').write_text(json.dumps(dict(archive='measurements.tar.gz',sha256=sha(archive.read_bytes()),bytes=archive.stat().st_size,members=len(inventory),readback_all_hashes_and_sizes_match=True),indent=2)+'\n')
for name in ['README.md','summary.json','totals.json','gate-summary.json','gates.json','scope-chronology.json','integrity-summary.json','build-provenance.json','diagnostic-build.json','java-source-audit.json','checker-additional-java-provenance.json','reference-validation.json','fresh-classification.json','fresh-independence.json','registered-pairs.json','residuals.json','causal-findings.json','whitespace-control.json','standing-summary.json','registration-effects.json','unit-residue.json','maker-output-reproduction.json','model-equality-observations.json','cwd-mapping.json','prepare_fresh.py','prepare_replay.py','run_replay.py','replay.py','setup.py','build.py','build_probe.py','validate_fresh.py','gates.py','audit.py','analyze.py','summarize.py','observations.py','final_records.py','package.py']:
 shutil.copy2(S/name,D/name)
print(json.dumps(json.loads((D/'archive-integrity.json').read_bytes()),indent=2))
