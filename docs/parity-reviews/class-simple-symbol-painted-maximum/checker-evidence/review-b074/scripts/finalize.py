# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,shutil,subprocess
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-review-b074');R=Path.cwd();C=R/'docs/parity-reviews/class-simple-symbol-painted-maximum/checker-evidence/review-b074';sha=lambda b:hashlib.sha256(b).hexdigest()
shutil.copy2(S/'evidence/perturbations.log',C/'perturbations.log')
rows=json.loads((C/'fresh28.json').read_text())['results']+json.loads((C/'supplement6.json').read_text())['results']
p=C/'fresh-summary.json';summary=json.loads(p.read_text());summary['model_retention']={'valid_reference_successful_yaml_models':28,'successful_yaml_outputs_including_java_negative_controls':30,'failed_yaml_outputs':4,'all_outputs_byte_identical_before_after':True,'interpretation':'Four empty parser-error outputs are not successful models. Valid retained model count is 28/30 valid references; two valid CommandUrl inputs fail parsing in both revisions.'};p.write_text(json.dumps(summary,indent=2)+'\n')
new={r['source_sha256'] for r in rows};old={}
for cohort in ['preserved1559','prior36','maker20']:old[cohort]={r['source_sha256'] for r in json.loads(gzip.decompress((C/f'{cohort}-before.json.gz').read_bytes()))['results']}
old['golden12550']={sha(p.read_bytes()) for p in (R/'test-diagrams/golden').rglob('*.puml') if p.with_suffix('.svg').exists()}
(C/'matrix-overlap.json').write_text(json.dumps({'new_entries':len(rows),'new_distinct_source_hashes':len(new),'new_intersection_with_existing':{name:len(new&hashes) for name,hashes in old.items()},'note':'Preserved1559 has explicit duplicates and overlapping smaller cohorts; no sums of overlapping inventories are used.'},indent=2)+'\n')
perturb=dict(command='CARGO_BUILD_JOBS=4 CARGO_TARGET_DIR=/Users/marcelo/Library/Caches/rustuml-review-b074/target-after RAYON_NUM_THREADS=4 RUST_TEST_THREADS=4 PLANTUML_STDLIB=/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100 cargo test --release --test parity_perturbations -- --nocapture',revision='b074d2082cb9ed2b9460f79a32b170e31cc1ab5e',exit=101,passed=1,failed=1,explanation='Missing/empty corpus guard passes; whole held-out parity test fails with historical and all new counterexamples preserved, including four Java-negative fixtures. Rejected reviews do not claim a green perturbation gate.',log=str((C/'perturbations.log').relative_to(R)))
(C/'perturbation-gate.json').write_text(json.dumps(perturb,indent=2)+'\n')
for m in ['class-simple-symbol-painted-maximum','class-note-url-group-ownership']:
 p=R/'docs/parity-reviews'/m/'review.json';r=json.loads(p.read_text());r['commands'].append(perturb['command']);r['perturbation_gate']=perturb;r['chronology'].append('Executed the standing parity_perturbations suite after installing every new pair: 1 passed/1 failed, exit101; rejection retained.');r['chronology'].append('Audited and manifested only new evidence/review paths for a single evidence-only commit; parent/maker files and all production remain immutable.');p.write_text(json.dumps(r,indent=2)+'\n')
# Record source state and original byte preservation before the evidence commit.
unchanged=subprocess.check_output(['git','diff','--name-only','b074d208','--','crates','bullseye.yaml','test-diagrams/no_oracle_baseline.txt','test-diagrams/ulp_parked.txt'],text=True);assert not unchanged
assert not subprocess.check_output(['git','diff','--name-only','b074d208'],text=True) # all requested evidence is newly added
assert not subprocess.check_output(['git','-C','test-diagrams/golden','status','--porcelain'],text=True)
(C/'scope-check.json').write_text(json.dumps({'tracked_file_changes_before_staging':[],'golden_commit':subprocess.check_output(['git','-C','test-diagrams/golden','rev-parse','HEAD'],text=True).strip(),'golden_clean':True,'rust_head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'rust_branch':subprocess.check_output(['git','branch','--show-current'],text=True).strip(),'no_production_or_historical_evidence_edits':True},indent=2)+'\n')
# Only new files can be in this manifest; hashes of historical data are retained in replay records.
files=[p for root in [C,R/'test-diagrams/perturbations/class-simple-symbol-painted-maximum',R/'test-diagrams/perturbations/class-note-url-group-ownership'] for p in root.rglob('*') if p.is_file() and p.name!='manifest-sha256.json']+[R/'docs/parity-reviews'/m/'review.json' for m in ['class-simple-symbol-painted-maximum','class-note-url-group-ownership']]
manifest={str(p.relative_to(R)):sha(p.read_bytes()) for p in sorted(files)};(C/'manifest-sha256.json').write_text(json.dumps(manifest,indent=2)+'\n')
print('manifest files',len(files),'models',summary['model_retention']);print('source overlaps',{name:len(new&hashes) for name,hashes in old.items()})
