# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,subprocess
from pathlib import Path
R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-5fc3056e');C=Path(__file__).resolve().parent;J=Path('/Users/marcelo/work/github.com/plantuml/plantuml');TIP='0cde5d7e55139b63cb8e91f8e42acd473441f80a';BASE='ae1e0de4d39840dd58074d89d40aa97304c6ca9f';IMP='82020a168c622ea109f1a61074c942072f37327e';PIN='71806a23780b04a5ccde2f8ceb5121edad5eb711'
def git(*a,cwd=R):return subprocess.check_output(['git',*a],cwd=cwd)
def tree(rev):
 return {x.split('\t')[1]:x.split()[2] for x in git('ls-tree','-r',rev).decode().splitlines()}
b,t=tree(BASE),tree(TIP);old=[p for p in b if p.startswith(('docs/parity-reviews/','test-diagrams/perturbations/'))];imported=git('diff-tree','--no-commit-id','--name-only','-r',IMP).decode().splitlines();i=tree(IMP)
record=dict(baseline_paths=len(old),baseline_changed=[p for p in old if t.get(p)!=b[p]],imported_paths=len(imported),imported_changed=[p for p in imported if t.get(p)!=i[p]],import_patch_identical=git('diff',IMP+'^',IMP)==git('diff','cd4a5ab71d68aaaa8915bb89a2a5450151609b68^','cd4a5ab71d68aaaa8915bb89a2a5450151609b68'),production_changed=git('diff','--name-only','653b1ed7a49784e6e6205f3aebf69a16278f8d78','d714bb55d40ca18590d15a5ffd57c7620b91b456','--','crates','Cargo.toml','Cargo.lock','scripts','bullseye.yaml').decode().splitlines())
record['accounts']=[]
for m,impl in [('sequence-external-create-consumer','f3d8bf0cce9caecfcaee445eaa69226f68378047'),('sequence-queue-complete-stereotype-display','d714bb55d40ca18590d15a5ffd57c7620b91b456')]:
 account='6857e26f9b0e1fc4c96d34faca06f7fe3ce539f2'
 record['accounts'].append(dict(mechanism=m,account=account,implementation=impl,precedes=subprocess.run(['git','merge-base','--is-ancestor',account,impl],cwd=R).returncode==0,implementation_message=git('show','-s','--format=%B',impl).decode(),account_unchanged=git('show',account+':docs/parity-reviews/'+m+'/account.json')==git('show',TIP+':docs/parity-reviews/'+m+'/account.json')))
paths=['sequencediagram/SequenceDiagram.java','sequencediagram/MessageExo.java','sequencediagram/Participant.java','sequencediagram/graphic/DrawableSetInitializer.java','sequencediagram/graphic/Step1MessageExo.java','sequencediagram/graphic/Step1Message.java','sequencediagram/teoz/CommunicationExoTile.java','sequencediagram/teoz/CommunicationTile.java','skin/AbstractTextualComponent.java','skin/rose/ComponentRoseQueue.java','decoration/symbol/USymbolQueue.java','klimt/creole/legacy/CreoleParser.java','klimt/creole/legacy/CreoleStripeSimpleParser.java','klimt/creole/legacy/StripeSimple.java','sequencediagram/command/CommandExoArrowLeft.java','sequencediagram/command/CommandExoArrowRight.java','sequencediagram/command/CommandExoArrowAny.java','sequencediagram/command/CommandReturn.java']
record['java_sources']=[]
for rel in paths:
 p='src/main/java/net/sourceforge/plantuml/'+rel;data=git('show',PIN+':'+p,cwd=J);record['java_sources'].append(dict(path=p,sha256=hashlib.sha256(data).hexdigest(),working_file_matches_pinned=data==(J/p).read_bytes()))
manifest=json.loads((C/'all-inputs.json').read_text());fresh=json.loads((C/'reference-manifest.json').read_text())['cases'];hashes={hashlib.sha256(git('show',TIP+':'+x['source'])).hexdigest() for x in manifest}
record['fresh_source_overlap']=[x['name'] for x in fresh if x['source_sha256'] in hashes]
record['golden_archive_revision']='9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a'
record['golden_files']=len(list((R/'test-diagrams/golden').rglob('*.puml')))
gold=Path('/Users/marcelo/work/github.com/marcelocantos/rustuml/test-diagrams/golden');entries=git('ls-tree','-r','9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a',cwd=gold).decode().splitlines();missing=[];changed=[]
for entry in entries:
 info,path=entry.split('\t');blob=info.split()[2];p=R/'test-diagrams/golden'/path
 if not p.exists():missing.append(path);continue
 data=p.read_bytes();actual=hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()
 if actual!=blob:changed.append(path)
record['golden_archive_verification']=dict(entries=len(entries),missing=missing,changed=changed)
(C/'source-audit.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps({k:v for k,v in record.items() if k not in ['java_sources','accounts']},indent=2))
