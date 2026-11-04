# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,subprocess
from pathlib import Path
R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-cb14b5f3');S=Path('/Users/marcelo/Library/Caches/rustuml-review-cb14b5f3');J=Path('/Users/marcelo/work/github.com/plantuml/plantuml');rev='71806a23780b04a5ccde2f8ceb5121edad5eb711';sha=lambda b:hashlib.sha256(b).hexdigest()
files=['tim/TContext.java','tim/TFunctionImpl.java','tim/TMemoryLocal.java','tim/VariableManager.java','tim/EaterFunctionCall.java','tim/EaterAffectationDefine.java','tim/EaterForeach.java','tim/EaterStartsub.java','tim/expression/ShuntingYard.java','tim/expression/ReversePolishInterpretor.java','tim/expression/TValue.java','tim/expression/TokenOperator.java','tim/iterator/CodeIteratorForeach.java','tim/iterator/CodeIteratorSub.java','tim/builtin/Substr.java','tim/builtin/Strpos.java','tim/builtin/Dec2hex.java','tim/builtin/Getenv.java','tim/builtin/SetVariableValue.java','tim/builtin/Chr.java','tim/builtin/Upper.java','tim/builtin/Lower.java','tim/builtin/Size.java','tim/builtin/Strlen.java','tim/builtin/BoolVal.java','preproc/Sub.java','preproc/UncommentReadLine.java','security/SecurityProfile.java','text/TLineType.java']
proof=[]
for name in files:
 path='src/main/java/net/sourceforge/plantuml/'+name;b=(J/path).read_bytes();frozen=subprocess.check_output(['git','show',rev+':'+path],cwd=J);assert b==frozen;proof.append(dict(path=path,sha256=sha(b)))
production=['Cargo.toml','Cargo.lock']+subprocess.check_output(['git','ls-tree','-r','--name-only','7bb210e9','--','crates','vendor'],cwd=R).decode().splitlines()
baseproof=[]
for path in production:
 p=S/'baseline-source'/path
 if p.is_file():
  raw=p.read_bytes();frozen=subprocess.check_output(['git','show','7bb210e9:'+path],cwd=R);assert raw==frozen;baseproof.append(dict(path=path,sha256=sha(raw)))
# Full frozen before/after production diff excludes evidence, and earlier date modules stayed unchanged.
assert not subprocess.check_output(['git','diff','--name-only','579b9b1173fef9dd3d10afca05b6d688b9bbd2c2','7bb210e9','--','crates','vendor','Cargo.toml','Cargo.lock'],cwd=R).strip()
changed=subprocess.check_output(['git','diff','--name-only','7bb210e9','ee0e4c72','--','crates','vendor','Cargo.toml','Cargo.lock'],cwd=R).decode().splitlines();assert changed==['crates/rustuml-parser/src/preprocess/mod.rs']
accounts=['8765ccc2ea4c2574fe90d25c2ba9921ad520682e','3020a0aaffba609cf48859ba3d65d21e7000a082','56a954287757fd23000d756b2fb483d43d7dcda2']
for account in accounts:subprocess.run(['git','merge-base','--is-ancestor',account,'6c550246b458eb5f37539713510dc7d919843421'],cwd=R,check=True)
frozen=['crates/rustuml-oracle/src/compare.rs','crates/rustuml-oracle/src/harness.rs','crates/rustuml-oracle/tests/harness_guards.rs','test-diagrams/no_oracle_baseline.txt','test-diagrams/ulp_parked.txt','bullseye.yaml']
record=dict(tested_revision='ee0e4c72fb222f1312d1aaf7d5a152cc97f12781',baseline_revision='7bb210e9f7578705638c8f3b7c3b43d8b4b906c3',fresh_baseline='Separately archived exact baseline Cargo/crates/vendor source, separately built baseline-target; no checkout or target mtime reuse',baseline_build_inputs=baseproof,baseline_cli_sha256=sha((S/'baseline-target/release/rustuml').read_bytes()),candidate_cli_sha256=sha((S/'candidate-rustuml').read_bytes()),comparator_binary_sha256=sha((S/'probe').read_bytes()),java_revision=rev,java_source_files=proof,new_production_paths=changed,precommitted_accounts=accounts,earlier_date_modules_unchanged=True,golden_gitlink=subprocess.check_output(['git','-C','test-diagrams/golden','rev-parse','HEAD'],cwd=R).decode().strip(),rustc=subprocess.check_output(['rustc','-Vv']).decode(),local_java_or_javac_launches=0,child_agents=0,source_audit='Whole preprocess/mod.rs read; fresh branches traced to cited pinned Java methods. No new fixture-label predicates, golden reads, coordinate surgery or unexplained parity constants found. Existing legacy helpers outside the changed mechanism were not promoted to fully audited parity.',frozen={p:sha((R/p).read_bytes()) for p in frozen})
(S/'provenance.json').write_text(json.dumps(record,indent=2)+'\n')
print('Java files byte-verified',len(proof),'baseline build inputs verified',len(baseproof))
