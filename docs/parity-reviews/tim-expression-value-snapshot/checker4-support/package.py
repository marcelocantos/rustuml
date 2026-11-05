# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,shutil,subprocess,tarfile
from pathlib import Path
R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-cb14b5f3');S=Path('/Users/marcelo/Library/Caches/rustuml-review-cb14b5f3');D=R/'docs/parity-reviews/tim-expression-value-snapshot/checker4-support';T=Path('/Users/marcelo/Library/Caches/rustuml-campaign-relay')
# Preserve receipts and reusable local-only tooling. No executables or target caches are committed.
files=['fresh-manifest.json','supplement-manifest.json','remote-request.json','supplement-request.json','relay-verification.json','supplement-relay-verification.json','fresh-candidate.json','fresh-baseline.json','supplement-candidate.json','supplement-baseline.json','fresh-summary.json','collateral-summary.json','controls.json','panic-groups.json','validation-basic.json','validation-main.json','validation-final.json','probe-build.json','maker17-reuse.json','builtin-versus-cli.json','generate.py','prepare.py','replay.py','evaluate.py','evaluate_supplement.py','analyze.py','validate.py','records.py','provenance.py','package.py']
for name in files:shutil.copy2(S/name,D/name)
for name in ['provenance.json','preservation-initial.json','collateral-manifest.json','collateral-results.json','baseline-results.json','collateral-source-models.json','membership.json','builtin-results.json']:
 (D/(name+'.gz')).write_bytes(gzip.compress((S/name).read_bytes(),mtime=0))
for name in ['tim29-results.json.gz','tim2-results.json.gz']:shutil.copy2(T/name,D/name)
with tarfile.open(D/'measurement-artifacts.tar.gz','w:gz') as t:
 for name in ['logs','outputs','fresh-reference','fresh-outputs','supplement-reference','supplement-outputs','file-controls','class-controls']:
  t.add(S/name,arcname=name)
 for name in ['production.diff']:t.add(S/name,arcname=name)
# All tracked starting-tree content is unchanged, including frozen references and old reviews.
assert not subprocess.check_output(['git','diff','--name-only','HEAD'],cwd=R).strip()
subprocess.run(['git','diff','--check'],cwd=R,check=True)
tracked=subprocess.check_output(['git','ls-files','docs/parity-reviews','test-diagrams/perturbations'],cwd=R).decode().splitlines()
status=dict(base_revision='ee0e4c72fb222f1312d1aaf7d5a152cc97f12781',all_starting_tree_tracked_files_unchanged=True,starting_evidence_files_preserved=len(tracked),new_review_records=7,new_sources=31,restored_prior_pairs=411,production_edits=0,local_java_or_javac=0,children=0,ship_actions=0)
(D/'preservation-final.json').write_text(json.dumps(status,indent=2)+'\n')
manifest=[]
for p in sorted(D.rglob('*')):
 if p.is_file() and p.name!='artifact-hashes.json':manifest.append(dict(path=str(p.relative_to(R)),bytes=p.stat().st_size,sha256=hashlib.sha256(p.read_bytes()).hexdigest()))
(D/'artifact-hashes.json').write_text(json.dumps(manifest,indent=2)+'\n')
print('support files',len(manifest),'archive bytes',(D/'measurement-artifacts.tar.gz').stat().st_size)
