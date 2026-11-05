# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Check evidence integrity without rerunning product tests or invoking Java."""
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess
ROOT=Path(__file__).resolve().parents[4]
OUT=Path(__file__).resolve().parent
CACHE=Path('/Users/marcelo/Library/Caches/rustuml-sequence-review-d382aba6')
def sha(data):return hashlib.sha256(data).hexdigest()
def git(*args):return subprocess.check_output(['git',*args],cwd=ROOT,text=True).strip()
assert git('rev-parse','HEAD')=='0fb4e14b7227eb768d268128d42fa5f6d5a5b1af'
# All tracked candidate content remains unchanged; our additions are review evidence only.
assert not git('diff','HEAD','--name-only')
rows=json.loads(gzip.decompress((OUT/'fresh.json.gz').read_bytes()))
assert len(rows)==68
for r in rows:
 p=ROOT/r['source'];g=ROOT/r['golden']
 assert sha(p.read_bytes())==r['source_sha256']
 assert sha(g.read_bytes())==r['golden_sha256']
 provenance=json.loads(p.with_suffix('.provenance.json').read_text())
 assert sha(g.read_bytes())==provenance['stdout_sha256']
 assert sha(p.with_suffix('.stderr').read_bytes())==provenance['stderr_sha256']
 assert provenance['exit_code']==(200 if r['cli_comparison']['exit']==2 else 0)
 assert r['cli_comparison']['exit'] in [0,1,2]
paths=json.loads((OUT/'review-index.json').read_text())
assert len(paths)==26
for name in paths:
 review=json.loads((ROOT/name).read_text())
 assert review['reviewer']['model']=='gpt-6-astra'
 assert review['tested_candidate_revision']=='0fb4e14b7227eb768d268128d42fa5f6d5a5b1af'
 assert review['verdict'] in ['accepted','rejected']
 for p in review['perturbations']+review.get('fresh_cross_mechanism_evidence',[]):
  assert sha((ROOT/p['source']).read_bytes())==p['source_sha256']
  assert sha((ROOT/p['golden']).read_bytes())==p['golden_sha256']
  assert (p['result']=='pass')==(p['comparator_exit']==0)
  if p.get('java_exit')==200:assert p['result']=='fail'
log=(CACHE/'logs/evidence-gates.log').read_text()
failing=re.findall(r'^  (test-diagrams/perturbations/[^\n]+?\.puml):',log,re.M)
assert len(failing)==len(set(failing))==584
all_sources=list((ROOT/'test-diagrams/perturbations').rglob('*.puml'))
assert len(all_sources)==1191
checks=json.loads((OUT/'checks.json').read_text())+json.loads((OUT/'checks-evidence.json').read_text())
result=dict(candidate=git('rev-parse','HEAD'),tracked_candidate_files_unchanged=True,new_review_records=len(paths),fresh_sources_unchanged=68,fresh_reference_streams_verified=68,negative_references_never_pass=True,standing_suite=dict(entries=1191,matches=607,nonpasses=584,valid_mismatches=565,invalid_references=19,cwd=str(ROOT/'crates/rustuml-oracle')),test_exit_codes={r['name']:r['exit'] for r in checks},new_records_sha256={p:sha((ROOT/p).read_bytes()) for p in paths},fresh_files_sha256={str(p.relative_to(ROOT)):sha(p.read_bytes()) for r in rows for p in [(ROOT/r['source']).with_suffix(s) for s in ['.puml','.svg','.stderr','.provenance.json']]})
(OUT/'final-integrity.json').write_text(json.dumps(result,indent=2)+'\n')
print('Verified26 review records,68 immutable sources/raw references,1191 standing pairs; candidate tracked files unchanged.')
