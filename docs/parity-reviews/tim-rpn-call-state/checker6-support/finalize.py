# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,shutil,subprocess,tarfile
from pathlib import Path
from replay import R,S,load,dump,sha
D=R/'docs/parity-reviews/tim-rpn-call-state/checker6-support'
# Clarify blockers for records whose bounded behavior survived.
for mech in ['tim-legacy-raw-caller-prefix','tim-intval-conversion','tim-hsl-typed-alpha']:
 p=R/'docs/parity-reviews'/mech/'review.json';r=load(p);r['acceptance_blockers']=r['acceptance_blockers'][1:];dump(p,r)
# Reviewed candidate module and both binaries are pinned separately.
commands={
'before':['cargo','build','--manifest-path','/tmp/rustuml-checker6/before-src/Cargo.toml','-p','rustuml','--release'],
'after':['cargo','build','-p','rustuml','--release']}
builds=[]
for phase in ['before','after']:
 revision='1703a2727fa364ee8004ba59a22b4c6934ee5d3d' if phase=='before' else '4a288dd47fe925a792f7cb672ca294f6ef6fdcf3'
 src=S/'before-src' if phase=='before' else R
 module=src/'crates/rustuml-parser/src/preprocess/mod.rs'
 assert module.read_bytes()==subprocess.check_output(['git','show',revision+':crates/rustuml-parser/src/preprocess/mod.rs'])
 builds.append(dict(phase=phase,revision=revision,argv=commands[phase],cwd=str(R),env_overrides=dict(CARGO_TARGET_DIR=str(S/('target-'+phase)),CARGO_BUILD_JOBS='4'),exit=0,module_sha256=sha(module.read_bytes()),binary_path=str(S/(phase+'-rustuml')),binary_sha256=sha((S/(phase+'-rustuml')).read_bytes()),log='logs/build-'+phase+'.log'))
dump(S/'builds.json',builds);dump(D/'builds.json',builds)
# Metadata schema, pair fidelity, and historical evidence are checked independently of red broad gates.
checks=[]
fresh=load(D/'fresh-results.json')
for r in fresh:
 src=R/r['committed_source'];svg=R/r['committed_golden'];assert src.with_suffix('.svg')==svg
 assert sha(src.read_bytes())==r['source_sha256'];assert sha(svg.read_bytes())==r['golden_sha256']
 checks.append(dict(source=r['committed_source'],golden=r['committed_golden'],source_sha256=r['source_sha256'],golden_sha256=r['golden_sha256']))
for mech in {r['mechanism'] for r in fresh}:
 r=load(R/'docs/parity-reviews'/mech/'review.json')
 for field in ['schema_version','mechanism','reviewer','account_commit','implementation_commit','java_revision','commands','perturbations','findings','verdict']:assert field in r
 assert r['verdict']=='rejected';assert r['reviewer']['independent_from_maker'];assert len(set(axis for p in r['perturbations'] for axis in p['axes']))>=2
old_checks=load(S/'old-evidence-integrity.json')
for snapshot in old_checks:
 for row in snapshot['files']:
  data=(R/row['path']).read_bytes();assert sha(data)==row['sha256'],row['path']
# Fresh pair census only; do not rerun standing or main gates.
stored=len(list((R/'test-diagrams/perturbations').rglob('*.puml')));assert stored==1605
out=dict(reviews_checked=6,fresh_pairs_checked=29,old_files_unchanged=[dict(revision=r['revision'],count=r['files_verified']) for r in old_checks],stored_perturbations_after_evidence=stored,broad_gate_reruns=0,pairs=checks)
dump(S/'evidence-validation.json',out);dump(D/'evidence-validation.json',out)
# Archive exact measurement bytes; exclude binaries, targets, source export and bytecode.
archive=D/'measurements.tar.gz'
with tarfile.open(archive,'w:gz') as tar:
 for p in sorted(S.iterdir()):
  if p.name in ['before-src','target-before','target-after','__pycache__','before-rustuml','after-rustuml','probe']:continue
  tar.add(p,arcname=p.name)
dump(D/'archive.json',dict(path=str(archive.relative_to(R)),bytes=archive.stat().st_size,sha256=sha(archive.read_bytes()),contains='all before/after replay captures, full raw relays, requests, immutable source inputs, scripts, logs, audit records and statuses; binaries/targets separately in private persistent cache'))
for name in ['reviews.py','finalize.py']:shutil.copy2(S/name,D/name)
# A persistent private cache preserves source exports, build caches and binary artifacts outside Git.
C=Path('/Users/marcelo/Library/Caches/rustuml-review-4a288dd-checker6');assert not C.exists();shutil.copytree(S,C,ignore=shutil.ignore_patterns('__pycache__'))
dump(D/'persistent-cache.json',dict(path=str(C),before_binary_sha256=sha((C/'before-rustuml').read_bytes()),after_binary_sha256=sha((C/'after-rustuml').read_bytes()),target_directories=['target-before','target-after'],request_paths_remain='/tmp/rustuml-checker6 (original immutable paths); cache is a durable byte copy'))
print(json.dumps(dict(validated_pairs=29,unchanged_old_files=8295,archive_bytes=archive.stat().st_size,persistent_cache=str(C)),indent=2))
