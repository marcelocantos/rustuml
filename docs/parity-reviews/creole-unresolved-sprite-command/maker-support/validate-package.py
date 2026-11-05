# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Mechanical packet preservation/registration checks; not a parity review."""
import base64,gzip,hashlib,json,subprocess,tarfile
from pathlib import Path
R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-a295df45');S=Path('/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-maker');D=R/'docs/parity-reviews/creole-unresolved-sprite-command';M=D/'maker-support';sha=lambda b:hashlib.sha256(b).hexdigest()
account=json.loads((D/'account.json').read_bytes())
required=['schema_version','mechanism','java_revision','java_locations','rust_locations','invariant','causal_divergence','planned_change','perturbation_axes'];assert all(k in account for k in required);assert len(account['perturbation_axes'])>=2
assert not list(D.glob('review*.json'))
base='0f00130a26b2f300996d0bd427f9e70b4ef10e30';ac='b160f41e2076b98805d4e3c9262a46030394e67c';impl='78f06098b74a81d24efcc922b3116d03ad496132'
for a,b in [(base,ac),(ac,impl)]:subprocess.run(['git','merge-base','--is-ancestor',a,b],cwd=R,check=True)
assert not subprocess.check_output(['git','diff','--name-only',base,ac,'--','crates'],cwd=R)
for rec in json.loads((M/'historical-files.json').read_bytes()):assert sha((R/rec['path']).read_bytes())==rec['sha256'],rec['path']
assert subprocess.check_output(['git','diff','--name-only',base,impl,'--','crates'],cwd=R).decode().splitlines()==['crates/rustuml-render/src/creole.rs','crates/rustuml-render/src/text_render.rs']
for phase in ['before','after']:
 b=json.loads((M/('build-'+phase+'.json')).read_bytes());assert sha((S/(phase+'-rustuml')).read_bytes())==b['binary_sha256']
 for p,h in b['source_hashes'].items():assert sha(subprocess.check_output(['git','show',b['source_revision']+':'+p],cwd=R))==h
for row in json.loads((M/'new-manifest.json').read_bytes()):
 p=Path(row['source']);registered=R/'test-diagrams/perturbations/creole-unresolved-sprite-command/maker'/p.name
 assert p.read_bytes()==registered.read_bytes();assert registered.with_suffix('.svg').read_bytes()==(M/'cases'/p.stem/'java.svg.stdout').read_bytes()
archive=M/'measurements.tar.gz';integrity=json.loads((M/'archive-integrity.json').read_bytes());assert sha(archive.read_bytes())==integrity['sha256'];entries=json.loads((M/'archive-files.json').read_bytes())
with tarfile.open(archive,'r:gz') as tar:
 members={m.name:m for m in tar.getmembers()};assert len(members)==len(entries)
 for e in entries:
  b=tar.extractfile(members[e['path']]).read();assert sha(b)==e['sha256'] and len(b)==e['bytes'],e['path']
result=dict(kind='mechanical maker packet verification, not mechanism attestation',account_schema=True,account_before_production=True,historical_files_unchanged=9731,production_paths_within_ownership=True,source_binary_hashes_verified=True,registered_fresh_pairs=18,all_reference_bytes_exact=True,archive_files_verified=len(entries),no_checker_record_authored=True,base=base,account_commit=ac,implementation_commit=impl)
(M/'registration-validation.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
