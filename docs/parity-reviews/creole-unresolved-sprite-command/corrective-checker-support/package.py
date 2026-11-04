# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import pathlib,json,tarfile,hashlib,shutil,subprocess
S=pathlib.Path(__file__).parent;R=pathlib.Path.cwd();D=R/'docs/parity-reviews/creole-unresolved-sprite-command/corrective-checker-support'
sha=lambda b:hashlib.sha256(b).hexdigest()
# Outputs are written to this private cache until intentionally packaged as review evidence.
exclude={'before-rustuml','after-rustuml','probe','line-probe-before','line-probe-after','measurements.tar.gz','archive-files.json','archive-integrity.json'}
files=[p for p in sorted(S.rglob('*')) if p.is_file() and p.name not in exclude and not any(v.startswith('target-') or v=='__pycache__' for v in p.relative_to(S).parts)]
rows=[dict(path=str(p.relative_to(S)),bytes=p.stat().st_size,sha256=sha(p.read_bytes())) for p in files]
archive=S/'measurements.tar.gz'
with tarfile.open(archive,'w:gz') as tar:
 for p in files:tar.add(p,arcname=str(p.relative_to(S)),recursive=False)
expected={r['path']:r for r in rows};verified=[]
with tarfile.open(archive,'r:gz') as tar:
 members=tar.getmembers();assert len(members)==len(expected)
 for m in members:
  assert m.isfile() and m.name in expected and '..' not in pathlib.PurePosixPath(m.name).parts and not m.name.startswith('/')
  b=tar.extractfile(m).read();assert len(b)==expected[m.name]['bytes'] and sha(b)==expected[m.name]['sha256'];verified.append(m.name)
(S/'archive-files.json').write_text(json.dumps(rows,indent=2)+'\n')
record=dict(members=len(rows),all_members_verified=True,archive_sha256=sha(archive.read_bytes()),archive_bytes=archive.stat().st_size,excluded='Rebuildable target directories, executables, Python bytecode caches; every raw measurement and stream retained.')
(S/'archive-integrity.json').write_text(json.dumps(record,indent=2)+'\n')
# Keep detailed captures in the archive; expose summaries, provenance and all replay scripts.
exposed=['summary.json','audit-summary.json','historical-audit.json','additional-java-source-audit.json','build-provenance.json','gates.json','standing-summary.json','standing-partitions.json','markup-controls.json','model-observations.json','text-observations.json','fresh-classification.json','manifest.json','request.json','reference-validation.json','reference-transport.json','parent-upload-validation.json','parent-relay-validation.json','independent-mapping-validation.json','line-probe.json','unit-residue.json','archive-files.json','archive-integrity.json','measurements.tar.gz']
for name in exposed:shutil.copy2(S/name,D/name)
for p in S.iterdir():
 if p.suffix in ['.py','.rs']:shutil.copy2(p,D/p.name)
# Check candidate production and protected evidence without rerunning broad product gates.
assert subprocess.check_output(['git','diff','a84856637adcd8e8d4e78fd2ee00658c9c612feb','--','crates','bullseye.yaml','test-diagrams/no_oracle_baseline.txt'])==b''
for r in json.loads((R/'docs/parity-reviews/creole-unresolved-sprite-command/maker-support/historical-files.json').read_bytes()):assert sha((R/r['path']).read_bytes())==r['sha256']
for r in json.loads((S/'historical-audit.json').read_bytes())['imported']:assert sha((R/r['path']).read_bytes())==r['sha256']
for mechanism in ['creole-command-candidate-traversal','creole-present-empty-stripe','wbs-semantic-text-measurement']:
 review=json.loads((R/'docs/parity-reviews'/mechanism/'review.json').read_bytes());assert review['verdict']=='rejected'
 for row in review['perturbations']:
  for key,h in [('source','source_sha256'),('golden','golden_sha256')]:assert sha((R/row[key]).read_bytes())==row[h]
print(json.dumps(record))
