# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,subprocess
from pathlib import Path
R=Path.cwd();C=Path(__file__).parent;O=C/'independent'; J=Path('/Users/marcelo/work/github.com/plantuml/plantuml')
BASE='0cde5d7e55139b63cb8e91f8e42acd473441f80a';TIP='bce6bea405d8b5efd55136f4768117115e7f634f';PROD='61e6aeed6eaf686b3497fbc696aadc03a64e69a2';PRE='d714bb55d40ca18590d15a5ffd57c7620b91b456';IMPORT='bd40c0f16838359779e406592edb51f32be2e9bd';LOCAL='7dcfb706517ed2a4af4601352e43471f45ae4973';sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*a,cwd=R):return subprocess.check_output(['git',*a],cwd=cwd)
def tree(rev):
 return {line.split(b'\t',1)[1].decode():line.split(b'\t',1)[0].decode() for line in git('ls-tree','-r',rev).splitlines()}
b,t,p=tree(BASE),tree(TIP),tree(PROD);old=[n for n in b if n.startswith(('docs/parity-reviews/','test-diagrams/perturbations/'))]
imports=git('diff-tree','--no-commit-id','--name-only','-r',IMPORT).decode().splitlines();data=dict(prior_paths=len(old),prior_differences=[n for n in old if b[n]!=t.get(n)],imported_paths=len(imports),import_differences=[n for n in imports if tree(IMPORT)[n]!=t.get(n)],import_commit_tree_delta_equal=git('diff',IMPORT+'^',IMPORT)==git('diff',LOCAL+'^',LOCAL),modified_existing=[n for n in b if n in t and b[n]!=t[n]],production_baseline_source_identical=all(v==tree(PRE).get(n) for n,v in b.items() if n.startswith('crates/')),production_final_source_identical=all(v==p.get(n) for n,v in t.items() if n.startswith('crates/')))
assert len(old)==9145 and len(imports)==205 and not data['prior_differences'] and not data['import_differences']
protected=O/'protected-paths.json.gz';protected.write_bytes(gzip.compress(json.dumps(dict(prior={n:b[n] for n in old},imported={n:t[n] for n in imports}),indent=2).encode(),mtime=0))
ascii=git('diff',BASE,TIP,'--','crates/rustuml-render/src/ascii.rs');data['ascii_exact_authorized_diff']=ascii==git('show',TIP+':docs/parity-reviews/sequence-filtered-display-packet/maker-results/ascii-authorized.diff');data['ascii_added_lines']=[x for x in ascii.decode().splitlines() if x.startswith('+') and not x.startswith('+++')]
(O/'production.diff').write_bytes(git('diff',BASE,PROD,'--','crates'))
mechanisms=['sequence-filtered-ordered-participant-display','sequence-note-aggregate-paint-height','sequence-external-note-owner','sequence-note-shape-terminator'];imps=['7c05cfcf8','fc71fafc7','aa5ac30e6','61e6aeed6'];accounts=['ca2aae93f']*3+['5d4858fb2'];data['causal_order']=[];java_paths=set()
for m,a,i in zip(mechanisms,accounts,imps):
 a=git('rev-parse',a).decode().strip();i=git('rev-parse',i).decode().strip();acct=json.loads(git('show',TIP+':docs/parity-reviews/'+m+'/account.json'))
 data['causal_order'].append(dict(mechanism=m,account=a,implementation=i,account_precedes_implementation_parent=subprocess.run(['git','merge-base','--is-ancestor',a,i+'^'],cwd=R).returncode==0,account_unchanged_since_precommit=git('show',a+':docs/parity-reviews/'+m+'/account.json')==git('show',TIP+':docs/parity-reviews/'+m+'/account.json'),implementation_message=git('show','-s','--format=%B',i).decode()))
 for loc in acct['java_locations']:
  file='src/main/java/net/sourceforge/plantuml/'+loc.split('#')[0].replace('.','/')+'.java';java_paths.add(file)
java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711';data['java_sources']=[dict(path=n,sha256=sha((J/n).read_bytes()),matches_pin=(J/n).read_bytes()==git('show',java_revision+':'+n,cwd=J)) for n in sorted(java_paths)]
data['java_head']=git('rev-parse','HEAD',cwd=J).decode().strip();data['golden_head']=git('rev-parse','HEAD',cwd=R/'test-diagrams/golden').decode().strip()
for n in ['crates/rustuml-oracle/src/compare.rs','crates/rustuml-oracle/src/harness.rs','crates/rustuml-oracle/tests/harness_guards.rs','docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py']:
 data.setdefault('frozen_tools',[]).append(dict(path=n,sha256=sha(git('show',TIP+':'+n)),unchanged=git('show',TIP+':'+n)==git('show',BASE+':'+n)))
# Read every maker JSON record, preserving exact compressed evidence hashes and structural inventories.
for n in ['creation-witnesses.json.gz','final-model-and-ascii.json.gz','remaining-failures.json.gz','cohort-overlap.json','protected-evidence.json','dependencies.json','model-ascii-yaml-summary.json']:
 raw=git('show',TIP+':docs/parity-reviews/sequence-filtered-display-packet/maker-results/'+n);v=json.loads(gzip.decompress(raw) if n.endswith('.gz') else raw);data.setdefault('maker_evidence_read',[]).append(dict(path=n,sha256=sha(raw),type=type(v).__name__,entries=len(v),keys=list(v) if isinstance(v,dict) else list(v[0]) if v else []))
(O/'static-audit.json').write_text(json.dumps(data,indent=2)+'\n');print(json.dumps(data,indent=2))
