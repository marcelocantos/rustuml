# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,subprocess
from pathlib import Path
R=Path.cwd();S=Path(__file__).resolve().parent;P=R/'docs/parity-reviews/class-description-positioned-stripes/maker-evidence/packet6'
def git(*args):return subprocess.check_output(['git',*args])
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((P/'manifest.json').read_text());report={}
report['manifest_hash_validation']={k:sha((P/k).read_bytes())==v for k,v in m['artifacts'].items()}
accounts=[]
for c in m['commits']:
 for f in c['changed_files']:
  if not f.endswith('/account.json'):continue
  b=(R/f).read_bytes();a=json.loads(b);accounts.append(dict(path=f,commit=c['commit'],sha256=sha(b),unchanged_since_account=git('show',c['commit']+':'+f)==b,ancestor_of_implementation=subprocess.run(['git','merge-base','--is-ancestor',c['commit'],'d84b89cd'],capture_output=True).returncode==0,implementation_message_cites_commit=c['commit'] in git('show','-s','--format=%B','d84b89cd').decode(),missing_schema_fields=[k for k in ['schema_version','mechanism','java_revision','java_locations','rust_locations','invariant','causal_divergence','planned_change','perturbation_axes'] if k not in a]))
report['accounts']=accounts
files=git('diff-tree','--no-commit-id','--name-only','-r','5416610690a5b507b6fd2e95a0cd30f780bd1192').decode().splitlines()
report['prior_independent_review_immutable']={f:git('show','5416610690a5b507b6fd2e95a0cd30f780bd1192:'+f)==(R/f).read_bytes() for f in files}
report['candidate_production_diff_from_implementation']=git('diff','--name-only','d84b89cd','eb09cecb','--','crates').decode().splitlines()
report['production_changed_paths']=git('diff','--name-only','b074d208','d84b89cd','--','crates').decode().splitlines()
report['comparator_tree_diff']=git('diff','--name-only','b074d208','eb09cecb','--','crates/rustuml-oracle').decode().splitlines()
report['golden_revision']=subprocess.check_output(['git','-C','test-diagrams/golden','rev-parse','HEAD'],text=True).strip()
report['golden_sources']=len(list((R/'test-diagrams/golden').rglob('*.puml')))
report['golden_worktree_status']=subprocess.check_output(['git','-C','test-diagrams/golden','status','--porcelain'],text=True)
report['binary_before_sha256']=sha((S/'target-before/release/rustuml').read_bytes())
(S/'audit.json').write_text(json.dumps(report,indent=2)+'\n')
print('accounts',len(accounts),'prior bytes',len(files),all(report['prior_independent_review_immutable'].values()),'artifact hashes',all(report['manifest_hash_validation'].values()))
for a in accounts:
 print(a['path'],a['missing_schema_fields'])
