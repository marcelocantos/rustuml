# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,shutil,subprocess,tarfile
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();D=R/'docs/parity-reviews/sequence-command-contract-repair-packet/checker-2b056ca1-astra';sha=lambda b:hashlib.sha256(b).hexdigest()
# Check every protected working-copy blob, not only tree-object equality.
p=json.loads((C/'protected-evidence.json').read_text());paths=sorted({x['path'] for x in p['paths']+p['imports']});expected={x['path']:x['evidence_blob'] for x in p['paths']+p['imports']}
actual=subprocess.check_output(['git','hash-object','--stdin-paths'],input=('\n'.join(paths)+'\n').encode()).decode().splitlines();assert len(actual)==len(paths);assert all(actual[i]==expected[path] for i,path in enumerate(paths));p['working_copy_verified_paths']=len(paths);p['working_copy_byte_exact']=True;(C/'protected-evidence.json').write_text(json.dumps(p,indent=2)+'\n')
assert not subprocess.check_output(['git','diff','--name-only','dc438fd0','--','crates','Cargo.toml','Cargo.lock','bullseye.yaml','test-diagrams/no_oracle_baseline.txt'])
assert subprocess.check_output(['git','-C','test-diagrams/golden','rev-parse','HEAD'],text=True).strip()=='9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a'
assert not subprocess.check_output(['git','diff','--name-only','b4849a500','dc438fd0','--','crates','Cargo.toml','Cargo.lock'])
assert not subprocess.check_output(['git','diff','--name-only','d491d4bda','9784e412','--','crates','Cargo.toml','Cargo.lock'])
# Independent structural validation of new evidence only; does not re-run or
# claim to pass the intentionally red repository review/decimal gates.
validation=[]
for entry in json.loads((C/'review-index.json').read_text()):
 r=json.loads((R/entry['path']).read_text());assert r['schema_version']==1 and r['verdict']=='rejected';assert len(r['perturbations'])>=2;assert len({a for x in r['perturbations'] for a in x['axes']})>=2
 for x in r['perturbations']:
  src=R/x['source'];svg=R/x['golden'];assert x['source'].startswith('test-diagrams/perturbations/'+r['mechanism']+'/');assert src.with_suffix('.svg')==svg;assert sha(src.read_bytes())==x['source_sha256'];assert sha(svg.read_bytes())==x['golden_sha256']
 validation.append(dict(path=entry['path'],shape_and_pair_hashes_valid=True,verdict='rejected'))
(C/'evidence-validation.json').write_text(json.dumps(dict(new_reviews=validation,protected_paths_unchanged=len(paths),production_unchanged=True,targets_and_baselines_unchanged=True,golden_revision='9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a',standing_pairs_before=2258,registered_new_pair_paths=424,registered_new_unique_source_bytes=422,registration_duplicates=2,standing_pairs_after=2682,broad_rerun_after_registration=False,guard_claim='Existing guards remain red; this is only shape/hash validation of new evidence.'),indent=2)+'\n')
# Visible compact records and reproducible support scripts; full raw captures in
# the archive exclude all compiled binaries and Cargo targets.
compact=['replay-summary.json','fresh-summary.json','gates-summary.json','review-index.json','source-audit.json','findings.json','reference-validation.json','intermediate-audit.json','quality-residue.json','evidence-validation.json','color-capture-derivation.json','registration-duplicates.json','novelty.json','before-build.json','after-build.json','rejection-commands.json','rejection-independent-command.json']
for n in compact:shutil.copy2(C/n,D/n)
for n in ['fresh-all.json','fresh-before-all.json','fresh-after-all.json','protected-evidence.json','rejection-historical.json','rejection-fresh.json','rejection-exporter.json','rejection-independent.json']:(D/(n+'.gz')).write_bytes(gzip.compress((C/n).read_bytes(),mtime=0))
for p in C.glob('*.py'):
 dest=D/'support'/p.name;dest.parent.mkdir(exist_ok=True);shutil.copy2(p,dest)
archive=C/'checker-captures.tar.gz'
with tarfile.open(archive,'w:gz') as t:
 for p in sorted(C.rglob('*')):
  rel=p.relative_to(C)
  if not p.is_file() or p==archive or any(x.endswith(('-target','-bin')) for x in rel.parts):continue
  t.add(p,arcname=str(rel),recursive=False)
shutil.copy2(archive,D/'captures.tar.gz')
(D/'archive.json').write_text(json.dumps(dict(file='captures.tar.gz',sha256=sha(archive.read_bytes()),size=archive.stat().st_size,contents='Raw CLI SVGs; complete AST/ASCII/YAML stdout/stderr base64/statuses; library diagnostics; exact before/after commands/logs/failure-name snapshots; immutable reference request/response/transport envelopes; first hydration failure and successful retry; source/hash audits. No compiled binaries/targets.'),indent=2)+'\n')
print('protected',len(paths),'archive',archive.stat().st_size,sha(archive.read_bytes()))
