# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,shutil,subprocess,tarfile
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();D=C/'durable-package';D.mkdir();DEST=Path('docs/parity-reviews/sequence-sprite-participant-packet/maker-results')
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def write(n,d):
 p=D/n;p.parent.mkdir(parents=True,exist_ok=True);raw=(json.dumps(d,indent=2)+'\n').encode();p.write_bytes(gzip.compress(raw,mtime=0) if p.suffix=='.gz' else raw)
def read(p):return json.loads(Path(p).read_text())
mapping={}
for folder in ['reference-inputs','completion-inputs']:
 for p in sorted((C/folder).iterdir()):
  q=D/'inputs'/p.name;q.parent.mkdir(exist_ok=True);shutil.copy2(p,q);mapping[str(p)]=str(DEST/'inputs'/p.name)
write('capture-path-mapping.json',mapping)
manifest=[]
for n in ['retained4140.json','maker30.json','completion4.json']:
 for r in read(C/n):
  for k in ['source','golden']:r[k]=mapping.get(r[k],r[k])
  manifest.append(r)
write('all-inputs.json',manifest)
for n in ['before','after']:
 rows=read(C/(n+'.json'))
 for r in rows:
  for k in ['source','golden']:r[k]=mapping.get(r[k],r[k])
 write(n+'.json.gz',rows)
for n in ['summary.json','cohort-summary.json','rejection-contract.json','changes.json','semantic-inventory.json','model-changes.json','unimplemented-command-residue.json','residue-summary.json','family-changes.json','remaining-failures.json','denominators.json','golden-byte-changes.json','golden-final-comparisons.json','golden-model-changes.json','golden-model-order-diagnostics.json','protected-evidence.json','parent-approved-exceptions.json','main-failure-name-comparison.json','standing-failures.json','implementation-map.json','intermediate-runs.json','dependencies.json','source-provenance.json','reference-classification.json','completion-reference-classification.json','final-checks.json','before-binding.json','final-binding.json']:
 if n in ['changes.json','semantic-inventory.json','golden-final-comparisons.json','golden-model-changes.json','golden-model-order-diagnostics.json','unimplemented-command-residue.json']:
  write(n+'.gz',read(C/n))
 else:shutil.copy2(C/n,D/n)
for prefix in ['', 'completion-']:
 for n in ['reference-results.json','reference-results.json.gz','reference-transport.json','reference-manifest.json','reference-input-bundle.json.gz','parent-relay-input.json.gz','parent-upload-validation.json','parent-relay-validation.json']:
  p=C/(prefix+n)
  if p.exists():shutil.copy2(p,D/p.name)
for p in C.glob('*.py'):shutil.copy2(p,D/p.name)
for folder in ['final-logs','final-failure-names','first-stable-logs','first-stable-failure-names']:
 for p in (C/folder).iterdir():
  q=D/'logs'/folder/(p.name+'.gz');q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(gzip.compress(p.read_bytes(),mtime=0))
for p in C.glob('*.log'):
 q=D/'logs'/(p.name+'.gz');q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(gzip.compress(p.read_bytes(),mtime=0))
for n in ['candidate1-binding.json','candidate2-binding.json','first-stable-binding.json','first-stable-path-mapping.json','candidate1-deltas.json']:
 shutil.copy2(C/n,D/n)
for name,argv in [('production.diff',['git','diff','2b8dc61e2','HEAD','--','crates']),('preprocess-authorized.diff',['git','diff','--unified=0','2b8dc61e2','HEAD','--','crates/rustuml-parser/src/preprocess/mod.rs'])]:
 (D/name).write_bytes(subprocess.check_output(argv))
# Every capture remains exact, including intermediate failures. Binary copies
# are excluded; exact SHA256/source bindings permit rebuilding them locally.
files=[]
for p in sorted(C.rglob('*')):
 rel=p.relative_to(C)
 if not p.is_file() or rel.parts[0]=='durable-package' or any(x.endswith('-bin') for x in rel.parts):continue
 files.append((p,rel))
write('capture-inventory.json',dict(files=[dict(path=str(rel),bytes=p.stat().st_size,sha256=sha(p)) for p,rel in files],excluded='Executable copies only; binary SHA256 bindings retained.',path_mapping='first-stable-path-mapping.json preserves renames of initial final-* captures after the justified pending-span correction. Original command strings remain unchanged.'))
with tarfile.open(D/'captures.tar.gz','w:gz') as tar:
 for p,rel in files:tar.add(p,arcname=str(rel),recursive=False)
write('archive.json',dict(path='captures.tar.gz',sha256=sha(D/'captures.tar.gz'),bytes=(D/'captures.tar.gz').stat().st_size,entries=len(files)))
print('staged',len(files),'captures', (D/'captures.tar.gz').stat().st_size,'bytes',flush=True)
