# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Preserve maker measurements, every outcome, and immutable reference bytes."""
import hashlib,json,shutil,subprocess,tarfile
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-maker');R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-a295df45');D=R/'docs/parity-reviews/creole-unresolved-sprite-command/maker-support';D.mkdir(exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
rows=[]
for name in ['membership.json','checker8-membership.json','sprites-membership.json']:rows+=json.loads((S/name).read_bytes())
paths=set()
for row in rows:
 for f in ['source_capture','golden_capture','reference_source_capture']:
  if f in row:paths.add(Path(row[f]))
 cwd=Path(row['cwd'])
 if not cwd.is_relative_to(R):
  paths.update(p for p in cwd.rglob('*') if p.is_file())
store=S/'retained-input-bytes';store.mkdir(exist_ok=True);mapping={}
for p in sorted(paths):
 b=p.read_bytes();digest=sha(b);(store/digest).write_bytes(b);mapping[str(p)]=dict(sha256=digest,bytes=len(b))
dump(S/'retained-input-paths.json',mapping)
for name in ['source_probe.rs']:
 shutil.copy2(Path('/Users/marcelo/Library/Caches/rustuml-tim8-checker')/name,S/name)
shutil.copy2(R/'docs/parity-reviews/tim-typed-values-and-binding/checker3-support/probe.rs',S/'probe.rs')
dump(S/'diagnostic-provenance.json',dict(comparator=dict(binary_sha256=sha((S/'probe').read_bytes()),source_sha256=sha((S/'probe.rs').read_bytes()),copied_from='/Users/marcelo/Library/Caches/rustuml-tim8-checker/probe',prior_build_record='docs/parity-reviews/tim-memory-trie-text-lookup/checker8-support/comparator-build.json'),source_probe=dict(binary_sha256=sha((S/'source-probe').read_bytes()),source_sha256=sha((S/'source_probe.rs').read_bytes()),copied_from='/Users/marcelo/Library/Caches/rustuml-tim8-checker/source-probe',prior_build_record='docs/parity-reviews/tim-memory-trie-text-lookup/checker8-support/source-probe-build.json'),scope='Unchanged comparator and preprocessing diagnostics. Actual product measurements use separately built source-only CLI executables.'))
for phase,revision in [('before','0f00130a26b2f300996d0bd427f9e70b4ef10e30'),('after','78f06098b74a81d24efcc922b3116d03ad496132')]:
 p=S/('build-'+phase+'.json');v=json.loads(p.read_bytes());v['source_revision']=revision;v['source_tree']=subprocess.check_output(['git','rev-parse',revision+'^{tree}'],cwd=R,text=True).strip();v['source_hashes']={x:sha(subprocess.check_output(['git','show',revision+':'+x],cwd=R)) for x in ['crates/rustuml-render/src/creole.rs','crates/rustuml-render/src/text_render.rs','crates/rustuml-parser/src/preprocess/mod.rs']};dump(p,v)
# New maker pairs join the unchanged standing corpus. Negative/failing outputs
# stay registered; there is intentionally no checker-authored review yet.
P=R/'test-diagrams/perturbations/creole-unresolved-sprite-command/maker';P.mkdir(parents=True,exist_ok=True)
for row in json.loads((S/'new-manifest.json').read_bytes()):
 p=Path(row['source']);g=S/'fresh-reference'/p.stem/'java.svg.stdout';shutil.copy2(p,P/p.name);shutil.copy2(g,P/(p.stem+'.svg'))
# Materialize useful small records and complete raw fresh results outside archive.
for name in ['prepare.py','replay.py','fresh-before.py','fresh-after.py','focused-collateral.py','fresh-standing.py','summarize.py','package.py','gates.py','remote-request.json','new-manifest.json','remote-validation.json','parent-relay-validation.json','reference-results.json.gz','summary.json','gates.json','integrity.json','build-before.json','build-after.json','diagnostic-provenance.json','fresh-before-results.json','fresh-after-results.json','fresh-standing.json','interruption.json','unit-residue.json','standing-mapping.json','historical-files.json']:
 shutil.copy2(S/name,D/name)
shutil.copytree(S/'fresh-reference',D/'cases',dirs_exist_ok=True)
excluded={'target-before','target-after','before-rustuml','after-rustuml','probe','source-probe','standing-slice','__pycache__'}
files=[p for p in sorted(S.rglob('*')) if p.is_file() and not any(part in excluded for part in p.relative_to(S).parts)]
manifest=[dict(path=str(p.relative_to(S)),sha256=sha(p.read_bytes()),bytes=p.stat().st_size) for p in files]
dump(D/'archive-files.json',manifest)
archive=D/'measurements.tar.gz'
with tarfile.open(archive,'w:gz',compresslevel=6) as tar:
 for p in files:tar.add(p,arcname=str(p.relative_to(S)),recursive=False)
with tarfile.open(archive,'r:gz') as tar:
 members={m.name:m for m in tar.getmembers()};assert len(members)==len(manifest)
 for entry in manifest:
  b=tar.extractfile(members[entry['path']]).read();assert sha(b)==entry['sha256'] and len(b)==entry['bytes']
dump(D/'archive-integrity.json',dict(archive='measurements.tar.gz',sha256=sha(archive.read_bytes()),bytes=archive.stat().st_size,files=len(manifest),all_archived_bytes_verified=True,excluded_rebuildable=sorted(excluded),retained_input_paths=len(mapping)))
print('archived',len(files),'files;',archive.stat().st_size,'bytes;',len(mapping),'retained resource/source/reference paths')
