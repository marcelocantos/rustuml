# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import subprocess,json,hashlib,re
from pathlib import Path
R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-78c595f4');C=Path(__file__).parent
TIP='ae1e0de4d39840dd58074d89d40aa97304c6ca9f';BASE='711dbca7c5e29592395f0927881aabfc2517b217';IMP='96e2561122e70ffe53dfa3f102cf18d199cc464f';LOCAL='2766d2c5425786df05cf62fed538003a6d2a52d3';ACC='4aaab169442b1e8a1d9e896d0cd3f019de6803d4'
def git(*a):return subprocess.check_output(['git',*a],cwd=R)
def tree(rev):
 return {p.decode():meta.decode().split()[2] for l in git('ls-tree','-r','-z',rev).split(b'\0') if l for meta,p in [l.split(b'\t',1)]}
b=tree(BASE);t=tree(TIP);i=tree(IMP)
prior={p:h for p,h in b.items() if p.startswith(('docs/parity-reviews/','test-diagrams/perturbations/'))}
import_paths=git('diff-tree','--no-commit-id','--name-only','-r',LOCAL).decode().splitlines()
changed=git('diff','--name-only',BASE,TIP,'--','crates').decode().splitlines()
accounts=json.loads(git('show',TIP+':docs/parity-reviews/sequence-pending-stripe-packet/maker-results/implementation-map.json'))['accounts']
chron=[]
for a in accounts:
 p='docs/parity-reviews/'+a['mechanism']+'/account.json'
 chron.append(dict(**a,first_commit=git('log','--reverse','--format=%H',TIP,'--',p).decode().splitlines()[0],preceded=all(subprocess.run(['git','merge-base','--is-ancestor',ACC,x],cwd=R).returncode==0 for x in a['implementation_commits']),unchanged_since_account=tree(ACC).get(p)==t[p],commit_messages=[git('show','-s','--format=%B',x).decode() for x in a['implementation_commits']]))
seq='crates/rustuml-render/src/sequence.rs'
def collection(rev):
 s=git('show',rev+':'+seq).decode();s=s[s.index('    fn collections_shape('):s.index('    fn queue_shape(')]
 return '\n'.join(l for l in s.splitlines() if not l.strip().startswith('//'))
java=Path('/Users/marcelo/work/github.com/plantuml/plantuml');jrev='71806a23780b04a5ccde2f8ceb5121edad5eb711';sources=['sequencediagram/SequenceDiagram.java','sequencediagram/Message.java','sequencediagram/MessageExo.java','sequencediagram/graphic/LifeLine.java','sequencediagram/teoz/LivingSpace.java','decoration/symbol/USymbolQueue.java','klimt/shape/TextBlockUtils.java','klimt/creole/SheetBlock1.java','klimt/creole/Sea.java','skin/rose/ComponentRoseArrow.java','skin/rose/ComponentRoseSelfArrow.java','skin/rose/Rose.java','skin/rose/ComponentRoseParticipant.java']
js=[]
for p in sources:
 p='src/main/java/net/sourceforge/plantuml/'+p;data=subprocess.check_output(['git','show',jrev+':'+p],cwd=java)
 js.append(dict(path=p,sha256=hashlib.sha256(data).hexdigest(),working_file_matches_pin=(java/p).read_bytes()==data))
report=dict(baseline=BASE,tip=TIP,prior_count=len(prior),prior_changes=[p for p,h in prior.items() if t.get(p)!=h],import_count=len(import_paths),import_changes=[p for p in import_paths if t.get(p)!=i.get(p)],production_changed=changed,chronology=chron,collections_noncomment_bytes_equal=collection(BASE)==collection(TIP),java_sources=js,prior_blobs=prior,import_blobs={p:i[p] for p in import_paths})
(C/'preservation-and-chronology.json').write_text(json.dumps(report,indent=2));print({k:v for k,v in report.items() if k not in ['prior_blobs','import_blobs','chronology','java_sources']})
