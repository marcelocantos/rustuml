# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import pathlib,json,hashlib,tarfile,os,shutil
S=pathlib.Path(__file__).parent;R=pathlib.Path('/Users/marcelo/.t3/worktrees/rustuml/t3-ae47bffd');D=R/'docs/parity-reviews/creole-color-resolution-fallback/checker-support';sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
# Read-only source snapshot and target artifacts are reproducible from pinned Git.
exclude={'src-before','src-after','target-before','target-after','before-rustuml','after-rustuml','probe','__pycache__','maker'}
files=[]
for root,dirs,names in os.walk(S):
 dirs[:]=sorted(x for x in dirs if x not in exclude)
 files.extend(pathlib.Path(root)/n for n in sorted(names) if n not in exclude)
files.sort();manifest=[dict(path=str(p.relative_to(S)),sha256=sha(p.read_bytes()),bytes=p.stat().st_size) for p in files]
dump(D/'archive-files.json',manifest);archive=D/'measurements.tar.gz'
with tarfile.open(archive,'w:gz',compresslevel=6) as t:
 for p in files:t.add(p,arcname=str(p.relative_to(S)),recursive=False)
with tarfile.open(archive,'r:gz') as t:
 members={x.name:x for x in t.getmembers()};assert len(members)==len(manifest)
 for row in manifest:
  b=t.extractfile(members[row['path']]).read();assert sha(b)==row['sha256'] and len(b)==row['bytes']
dump(D/'archive-integrity.json',dict(archive=archive.name,sha256=sha(archive.read_bytes()),bytes=archive.stat().st_size,files=len(files),all_members_read_back_and_verified=True,excluded_rebuildable=sorted(exclude),maker_exclusion='Original maker archive is already retained byte-exact at ../maker-support/measurements.tar.gz; extract it into maker/ to resolve frozen-input paths. No maker history omitted from the repository.'))
for p in sorted(S.glob('*.py')):shutil.copy2(p,D/p.name)
for name in ['audit-summary.json','archive-and-inventory-check.json','baseline-failure-name-check.json','builds.json','diagnostic-build.json','causal-findings.json','fresh-classification.json','fresh-independence.json','gate-summary.json','gates.json','java-source-audit.json','loss-error-summary.json','reference-validation.json','relay-independent-audit.json','registered-pairs.json','registry-source-check.json','residual-cause-check.json','setup-record.json','standing-summary.json','summary.json','no_oracle_failure_names.txt','golden_failure_names.txt','parent-relay-validation.json','parent-upload-validation.json']:
 shutil.copy2(S/name,D/name)
print('verified evidence archive',len(files),'members',archive.stat().st_size,'bytes',sha(archive.read_bytes()))
