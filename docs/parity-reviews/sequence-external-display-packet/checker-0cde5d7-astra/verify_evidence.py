# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,subprocess,tarfile
from pathlib import Path
R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-5fc3056e');C=Path(__file__).resolve().parent;D=R/'docs/parity-reviews/sequence-external-display-packet/checker-0cde5d7-astra'
sha=lambda b:hashlib.sha256(b).hexdigest()
index=json.loads((D/'review-index.json').read_text());sources=set();n=0
for item in index:
 review=json.loads((R/item['review']).read_text());assert review['verdict']=='rejected';assert review['tested_production']=='d714bb55d40ca18590d15a5ffd57c7620b91b456'
 for field in ['schema_version','mechanism','reviewer','account_commit','implementation_commit','java_revision','commands','perturbations','findings','verdict']:assert field in review
 for p in review['perturbations']:
  s=R/p['source'];g=R/p['golden'];assert s.with_suffix('.svg')==g;assert sha(s.read_bytes())==p['source_sha256'];assert sha(g.read_bytes())==p['reference_sha256'];assert p['source'] not in sources;sources.add(p['source']);n+=1
assert n==64
reproduction=json.loads((D/'maker-reproduction-and-overlap.json').read_text())
for rev in ['before','after']:
 for key in ['source_hash_differences','reference_hash_differences','svg_hash_differences','status_differences']:assert not reproduction[rev][key]
 assert all(not x for x in reproduction[rev]['comparison_differences'].values())
audit=json.loads((D/'source-audit.json').read_text());assert audit['baseline_paths']==8782 and not audit['baseline_changed'];assert audit['imported_paths']==245 and not audit['imported_changed'];assert audit['import_patch_identical'];assert all(x['working_file_matches_pinned'] for x in audit['java_sources']);assert not audit['golden_archive_verification']['missing'] and not audit['golden_archive_verification']['changed']
# Verify the archived raw captures, not just summaries; every replay entry retains both revisions and all three CLI modes.
counts={};expected_hashes={}
with tarfile.open(D/'captures.tar.gz') as t:
 for cohort,expected in [('replay2281',2281),('fresh60',60),('followup4',4)]:
  rows=json.loads(gzip.decompress(t.extractfile(cohort+'/measurements.json.gz').read()));assert len(rows)==expected
  for r in rows:
   for rev in ['before','after']:
    for mode in ['cli','ast','ascii']:
     x=r['results'][rev][mode];expected_hashes[cohort+'/'+x['output']]=x['sha256']
  counts[cohort]=len(rows)
with tarfile.open(D/'captures.tar.gz',mode='r|gz') as t:
 for member in t:
  if member.name in expected_hashes:assert sha(t.extractfile(member).read())==expected_hashes.pop(member.name)
assert not expected_hashes
for path in ['crates','Cargo.lock','Cargo.toml','bullseye.yaml','scripts']:
 assert not subprocess.check_output(['git','diff','0cde5d7e','--',path],cwd=R)
tracked=subprocess.check_output(['git','diff','--name-only'],cwd=R,text=True);assert not tracked,'Existing tracked content was edited: '+tracked
report=dict(validated_pairs=n,raw_capture_entries=counts,all_raw_cli_ast_ascii_hashes_verified=True,old_evidence_unchanged=True,production_unchanged=True,verdicts={x['mechanism']:x['verdict'] for x in index},note='Evidence integrity validation, not a passing product parity claim.')
(D/'integrity.json').write_text(json.dumps(report,indent=2)+'\n')
(D/'artifact-sha256.json').write_text(json.dumps({str(p.relative_to(D)):sha(p.read_bytes()) for p in sorted(D.rglob('*')) if p.is_file() and p.name!='artifact-sha256.json'},indent=2)+'\n')
print(json.dumps(report,indent=2))
