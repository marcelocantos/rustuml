# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,subprocess
from pathlib import Path
S=Path('/tmp/rustuml-checker6');R=Path.cwd();J=Path('/Users/marcelo/work/github.com/plantuml/plantuml');rev='71806a23780b04a5ccde2f8ceb5121edad5eb711'
def git(*a,cwd=R):return subprocess.check_output(['git',*a],cwd=cwd)
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
checks=[]
for base in ['079c1bc1ffc325e8ebfa1719bc0ce2d3c71573c2','4a288dd47fe925a792f7cb672ca294f6ef6fdcf3']:
 rows=[]
 for ent in git('ls-tree','-rz',base,'--','docs/parity-reviews','test-diagrams/perturbations').split(b'\0'):
  if not ent:continue
  meta,path=ent.split(b'\t');mode,kind,oid=meta.split();p=R/path.decode();data=p.read_bytes();actual=hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest();assert actual==oid.decode(),str(p)
  rows.append(dict(path=path.decode(),git_blob=actual,sha256=hashlib.sha256(data).hexdigest(),bytes=len(data)))
 checks.append(dict(revision=base,files_verified=len(rows),files=rows))
dump(S/'old-evidence-integrity.json',checks)
paths=[r['path'] for r in json.loads((R/'docs/parity-reviews/tim-rpn-call-state/maker-support/java-source-provenance.json').read_text())]
paths+=['src/main/java/net/sourceforge/plantuml/tim/TMemoryLocal.java','src/main/java/net/sourceforge/plantuml/tim/iterator/CodeIteratorProcedure.java','src/main/java/net/sourceforge/plantuml/tim/iterator/CodeIteratorReturnFunction.java','src/main/java/net/sourceforge/plantuml/tim/builtin/GetVariableValue.java']
rows=[]
for p in sorted(set(paths)):
 b=(J/p).read_bytes();expected=git('show',rev+':'+p,cwd=J);assert b==expected,p;rows.append(dict(path=p,sha256=hashlib.sha256(b).hexdigest(),bytes=len(b),revision=rev))
dump(S/'java-source-provenance.json',rows)
accounts=['tim-rpn-call-state','tim-call-operand-resolution','tim-legacy-raw-caller-prefix','tim-local-sub-collection','tim-intval-conversion','tim-hsl-typed-alpha']
for mech in accounts:
 p='docs/parity-reviews/'+mech+'/account.json';assert git('show','c91103f9:'+p)==(R/p).read_bytes()
for a,b in [('079c1bc1','c91103f9'),('c91103f9','f663c083'),('f663c083','4a288dd4')]:subprocess.run(['git','merge-base','--is-ancestor',a,b],check=True)
assert git('diff','f663c083','4a288dd4','--','crates','Cargo.toml','Cargo.lock')==b''
for p in ['crates/rustuml-oracle/src/compare.rs','crates/rustuml-oracle/src/harness.rs','docs/parity-reviews/tim-typed-values-and-binding/checker3-support/probe.rs']:
 assert git('show','1703a272:'+p)==(R/p).read_bytes(),p
(S/'production.diff').write_bytes(git('diff','c91103f9','f663c083','--','crates/rustuml-parser/src/preprocess/mod.rs'))
(S/'chronology.log').write_bytes(git('log','--format=fuller','--stat','c91103f9^..4a288dd4'))
dump(S/'audit.json',dict(accounts_unchanged=accounts,chronology='079c1bc1 < c91103f9 < f663c083 < 4a288dd4; ancestry and blobs checked',production_only_paths=git('diff','--name-only','c91103f9','f663c083').decode().splitlines(),production_equals_candidate=True,old_evidence_checks=[dict(revision=r['revision'],files=r['files_verified']) for r in checks],java_sources_verified=len(rows),golden_gitlink=git('rev-parse','HEAD',cwd=R/'test-diagrams/golden').decode().strip(),binaries={phase:hashlib.sha256((S/(phase+'-rustuml')).read_bytes()).hexdigest() for phase in ['before','after']},probe_sha256=hashlib.sha256((S/'probe').read_bytes()).hexdigest()))
print((S/'audit.json').read_text())
