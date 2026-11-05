# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Summarize measured evidence and verify preservation, without changing gates."""
import hashlib,json,re,subprocess,tarfile
from pathlib import Path
S=Path(__file__).resolve().parent;ROOT=S.parents[3];T=Path('/tmp/rustuml-tim-checker3.dRQvKH')
def load(p):return json.loads(p.read_text())
def dump(p,d):p.write_text(json.dumps(d,indent=2)+'\n')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def git(*args):return subprocess.check_output(['git',*args],cwd=ROOT)
prep=load(S/'preparation.json');inventory=load(S/'frozen-inventory.json')
for r in inventory:
 assert sha(ROOT/r['source'])==r['source_sha256']
 assert sha((ROOT/r['source']).with_suffix('.svg'))==r['golden_sha256']
tracked=git('ls-tree','-r','--name-only',prep['parent'],'docs/parity-reviews').decode().splitlines()
changed=[p for p in tracked if git('show',prep['parent']+':'+p)!=(ROOT/p).read_bytes()];assert not changed
production=git('diff',prep['parent'],prep['after'],'--name-only','--','crates').decode().splitlines()
assert all(p.startswith('crates/rustuml-parser/src/preprocess/') or p in ['crates/rustuml-parser/src/parse/mod.rs','crates/rustuml/tests/pipe_mode.rs'] for p in production)
assert not git('diff','6f52f87e532d549b047f7ccd3fe7fb577fc56df9',prep['after'],'--','crates/rustuml-parser/src/preprocess','crates/rustuml-parser/src/parse/mod.rs','crates/rustuml/tests/pipe_mode.rs')
frozen_paths=['bullseye.yaml','test-diagrams/no_oracle_baseline.txt','crates/rustuml-oracle/src/compare.rs','crates/rustuml-oracle/src/harness.rs','crates/rustuml-oracle/tests/harness_guards.rs']
for p in frozen_paths:assert git('show',prep['parent']+':'+p)==(ROOT/p).read_bytes()
binary={n:sha(T/(n+'-rustuml')) for n in ['before','after']}
dump(S/'preservation.json',dict(parent=prep['parent'],prepared=prep['after'],frozen_pairs=len(inventory),all_parent_review_files_unchanged=len(tracked),frozen_paths={p:sha(ROOT/p) for p in frozen_paths},production_paths=production,production_matches_requested_tip=True,binary_sha256=binary,comparator_source_sha256=sha(S/'probe.rs'),comparator_binary_sha256=sha(T/'probe'),golden_revision=git('-C','test-diagrams/golden','rev-parse','HEAD').decode().strip(),golden_is_symlink=(ROOT/'test-diagrams/golden').is_symlink()))
summary=dict(main_product=dict(passed=11179,eligible=11251,status='failed unchanged ratchets',runs=1),main_assisted=dict(passed=10497,failed=754,skipped=1299,total=12550,status='failed',runs=1),parser=dict(passed=582,failed=0),cli=dict(pipe_passed=15,pipe_failed=0,binary_unit_tests=0),guards=dict(passed=4,failed=2,failures=['decimal_literal_ratchet','parity_review_records_are_complete'],new_owned_schema_errors=[],candidate_support_residue='tim-corrective-execution-maker has no account.json; unmodified maker evidence directory is walked by the existing guard'),clippy=dict(status='failed',reason='pre-existing dead code parse/component.rs:72 looks_like_description_bracket_command'),touched_format=dict(status='passed',files=['preprocess/mod.rs','preprocess/debug.rs','preprocess/date_format.rs','parse/mod.rs','pipe_mode.rs']),builtin=load(S/'builtin-results.json'),comparisons=load(S/'deltas.json'),regression_controls=dict(total=11,svg=11,source=11,remote_vs_preserved=11),zone_names=dict(remote_run_exit=0,ids=len(load(T/'zone-names.json')),remote_sha256=sha(T/'zone-names.json'),production_sha256=sha(ROOT/'crates/rustuml-parser/src/preprocess/zone_names.json')))
dump(S/'verification-summary.json',summary)
dump(S/'reference-environment.json',dict(java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',host='marcelo@192.168.1.112',remote_scratch='/Users/marcelo/rustuml-reference/scratch/worker.l4Yx4xIn',wrapper='/Users/marcelo/rustuml-reference/bin/reference',new_worker_command='ssh -T -o BatchMode=yes marcelo@192.168.1.112 /Users/marcelo/rustuml-reference/bin/new-worker',all_java_remote=True,local_jvm_or_javac_invocations=0,readiness=load(T/'remote-readiness.json'),origin_runtime='Temurin21.0.13 / Graphviz16.1.0',remote_runtime='OpenJDK21.0.5 / Graphviz12.2.0',calibration_limit='Seven root calibration pairs only; eleven regression controls additionally rechecked here. Not universal environment equivalence.',locale='en_AU',encoding='UTF8',headless='forced by wrapper',timezone='GMT+11:00 unless row explicitly names another; matching local TZ/debug declared in manifests',cargo_target=str(T/'target'),cargo_build_jobs=4,local_rust_cwd=str(ROOT)))
# Raw subprocess products first lived in private /tmp; deliberately promote a bounded archive.
archive=S/'measurement-artifacts.tar.gz'
with tarfile.open(archive,'w:gz') as tar:
 for name in ['outputs','reference']:
  tar.add(T/name,arcname=name)
 for p in sorted(T.glob('*.log')):tar.add(p,arcname='logs/'+p.name)
 for name in ['remote-readiness.json','zone-names.json','zone-names.stderr']:tar.add(T/name,arcname=name)
files=[p for p in S.rglob('*') if p.is_file() and p.name!='artifact-hashes.json' and '__pycache__' not in str(p)]
dump(S/'artifact-hashes.json',{str(p.relative_to(S)):sha(p) for p in sorted(files)})
print('preserved',len(inventory),'pairs;',len(tracked),'parent review files; archive',archive.stat().st_size)
