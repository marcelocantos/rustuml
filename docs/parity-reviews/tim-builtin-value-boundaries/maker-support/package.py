# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Package maker observations without changing checker records or frozen data."""
import gzip,hashlib,json,shutil,subprocess,tarfile
from pathlib import Path
R=Path.cwd();S=Path('/tmp/rustuml-tim5');D=R/'docs/parity-reviews/tim-builtin-value-boundaries/maker-support';D.mkdir(parents=True,exist_ok=True)
def sha(b):return hashlib.sha256(b).hexdigest()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
small=['summary.json','iteration1-collateral-summary.json','iteration1-fresh31-summary.json','registration-audit.json','validation.json','controls.json','prior17-results.json','new-manifest.json','remote-request.json','new-rust-results.json','standing-versus-isolated.json']
large=['iteration1-collateral.json','iteration1-fresh31.json','collateral-source-results.json','source-models.json','frozen-membership.json','standing-results.json']
for name in small:shutil.copy2(S/name,D/name)
for name in large:(D/(name+'.gz')).write_bytes(gzip.compress((S/name).read_bytes(),mtime=0))
for name in ['measure.py','analyze.py','summarize.py','prior17.py','new_rust.py','validate.py','audit_registration.py','generate.py','package.py','relay_eval.py']:shutil.copy2(S/name,D/name)
shutil.copytree(S/'new',D/'inputs',dirs_exist_ok=True)
# Save every completed capture and log, including failed interim test and unfrozen-clock run.
with tarfile.open(D/'measurement-artifacts.tar.gz','w:gz') as t:
 for folder in ['outputs','fresh-outputs','prior17','controls','new-rust','relay15']:
  t.add(S/folder,arcname=folder)
 for p in sorted(S.glob('*.log')):t.add(p,arcname='logs/'+p.name)
# Verify existing tracked accounts/reviews/fixtures by Git blob identity in one process.
base='47a723b6fd424a2584dc3d89ac0dc0328eeb0ae7'
listing=subprocess.check_output(['git','ls-tree','-r',base,'docs/parity-reviews','test-diagrams']).decode().splitlines()
files=[]
for line in listing:
 meta,path=line.split('\t');mode,kind,oid=meta.split()
 if kind=='blob':files.append((path,oid))
q=subprocess.run(['git','hash-object','--stdin-paths'],input=''.join(p+'\n' for p,_ in files),text=True,capture_output=True,check=True)
actual=q.stdout.splitlines();assert len(actual)==len(files)
for (path,expected),current in zip(files,actual):assert current==expected,path
preservation=dict(base=base,tracked_historical_files_verified=len(files),changed_existing_files=[],pair_inventory_sha256=sha((S/'frozen-membership.json').read_bytes()),tree_inputs_count=1547,restored_prior_pairs=411,fresh_checker_pairs=31)
dump(D/'preservation.json',preservation)
provenance=dict(candidate=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),account_commit='6d46a483aad389d4535e1ba24822fdd116baf517',evidence_import=base,checker_evidence='0d30ae99e85195f595e21ca0a80be85f26473ee9',previous_candidate='ee0e4c72fb222f1312d1aaf7d5a152cc97f12781',previous_production='6c550246b458eb5f37539713510dc7d919843421',candidate_binary_sha256=sha((S/'iteration1-rustuml').read_bytes()),before_binary_sha256=sha((S/'before-rustuml').read_bytes()),comparator_sha256=sha((S/'probe').read_bytes()),production_module_sha256=sha((R/'crates/rustuml-parser/src/preprocess/mod.rs').read_bytes()),owned_target='/tmp/rustuml-tim5-target',before_target='/tmp/rustuml-tim-repair-target',build_jobs=4,rustc=subprocess.check_output(['rustc','-Vv'],text=True).strip(),cargo=subprocess.check_output(['cargo','-V'],text=True).strip(),golden_gitlink=subprocess.check_output(['git','rev-parse','HEAD:test-diagrams/golden'],text=True).strip(),java_source_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',new_java_capture_status='TIM15 completed through parent T3 relay:18 resources,30 remote calls;9 valid exact-source/SVG matches and6 negative references rejected. No local JVM.',local_java_launches=0,ssh_attempts=0,canonical_debug='date=1774210426000,tz=AEDT+1100',canonical_zone='GMT+11:00',remote_runtime='OpenJDK 21.0.5 / Graphviz 12.2.0 / UTF-8 / en_AU / headless; differs from origin 21.0.13 / 16.1.0. Scoped conformance only.',baseline_measurement='checker4 preserved candidate results, exact same 1516 sources/references; prior candidate binary additionally retained separately. New15 has actual before/current CLI captures.',headless_environment='JAVA_TOOL_OPTIONS existing value plus -Djava.awt.headless=true on cargo commands; no JVM launcher is called.',validation_correction='First standing gate omitted pinned RUSTUML_DEBUG and measured548/1547 with40 environment differences; log preserved. Only that gate repeated with pinned clock:562/1547 and26 expected environment differences. Main tiers each run once.',parser=dict(command=['cargo','test','-p','rustuml-parser','--lib'],exit=0,passed=597,log='logs/parser2.log'),artifact_sha256=sha((D/'measurement-artifacts.tar.gz').read_bytes()))
dump(D/'provenance.json',provenance)
print('packaged',D,'historical files unchanged',len(files),'artifact bytes',(D/'measurement-artifacts.tar.gz').stat().st_size)

shutil.copytree(S/'relay15',D/'relay15',dirs_exist_ok=True,ignore=shutil.ignore_patterns('before','iteration1'))
for result in json.loads((S/'relay15/results.json').read_bytes()):
 p=Path(result['source']);shutil.copy2(S/'relay15'/p.stem/'java.svg.stdout',D/'inputs'/p.with_suffix('.svg').name)
