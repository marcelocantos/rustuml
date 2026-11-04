# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,re,subprocess
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();J=Path('/Users/marcelo/work/github.com/plantuml/plantuml');JP=J/'src/main/java/net/sourceforge/plantuml';TIP='dc438fd0cd0f5edd44816db81f0d765bf48ce4e6';BASE='9784e412146e3ece634d88ea0a1bddd6b42a82b6';IMP='e654acb9ba31aec53329c739e8151d5bf46659f7';PROD='b4849a5008ff28c360fefe0176ee6ad073c02fd7'
def git(*a):return subprocess.check_output(['git',*a])
def obj(rev,p):return git('show',rev+':'+p)
def entries(rev):
 return {row.split('\t',1)[1]:row.split()[2] for row in git('ls-tree','-r',rev).decode().splitlines()}
sha=lambda b:hashlib.sha256(b).hexdigest();tip=entries(TIP);base=entries(BASE);imp=entries(IMP)
prior=[p for p in base if p.startswith(('docs/parity-reviews/','test-diagrams/perturbations/'))];imported=git('diff-tree','--no-commit-id','--name-only','-r',IMP).decode().splitlines()
assert len(prior)==10159 and len(imported)==1641
assert all(base[p]==tip[p] for p in prior);assert all(imp[p]==tip[p] for p in imported)
proof=dict(prior_paths=len(prior),imported_paths=len(imported),byte_exact=True,paths=[dict(path=p,baseline_blob=base[p],evidence_blob=tip[p]) for p in prior],imports=[dict(path=p,import_blob=imp[p],evidence_blob=tip[p]) for p in imported])
(C/'protected-evidence.json').write_text(json.dumps(proof,indent=2)+'\n')
mp=json.loads(obj(TIP,'docs/parity-reviews/sequence-command-contract-repair-packet/maker-results/implementation-map.json'));facts=[];citations=[]
for row in mp['mechanisms']:
 m=row['mechanism'];a=row['account_commit'];i=row['implementation_commit'];p='docs/parity-reviews/'+m+'/account.json';account=json.loads(obj(a,p));parent=git('rev-parse',i+'^').decode().strip()
 precedes=subprocess.run(['git','merge-base','--is-ancestor',a,parent]).returncode==0
 assert precedes and obj(a,p)==obj(TIP,p)
 facts.append(dict(**row,account_precedes_draft=precedes,account_unchanged=True,account_sha256=sha(obj(a,p)),implementation_message=git('show','-s','--format=%B',i).decode()))
 for loc in account['java_locations']:
  cls,methods=loc.split('#');f=JP/(cls.replace('.','/')+'.java');body=f.read_bytes(); pinned=subprocess.check_output(['git','-C',str(J),'show','71806a23780b04a5ccde2f8ceb5121edad5eb711:'+str(f.relative_to(J))]);assert body==pinned
  for method in methods.split('/'):
   matches=[dict(line=n,text=line.strip()) for n,line in enumerate(body.decode().splitlines(),1) if re.search(r'\b'+re.escape(method)+r'\s*\(',line) and re.search(r'\b(public|private|protected|static|final|IRegex)\b',line)]
   field=method=='QUOTED_REPLACEMENTS' and b'QUOTED_REPLACEMENTS =' in body
   citations.append(dict(mechanism=m,citation=cls+'#'+method,path=str(f),sha256=sha(body),pinned_byte_equal=True,exists=bool(matches) or field,locations=matches,field=field))
prod_changed=git('diff','--name-only',BASE,PROD,'--','crates').decode().splitlines()
assert prod_changed==['crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs']
# Enumerate character classes with whitespace plus other members (not the whole
# simple class); Java source lexical matches are checked manually below.
composite=[]
for directory in ['sequencediagram/command','command/note/sequence']:
 for f in (JP/directory).glob('*.java'):
  for n,line in enumerate(f.read_text().splitlines(),1):
   for cls in re.findall(r'\[[^\]]*%s[^\]]*\]',line):
    if cls not in ['[%s]','[^%s]']:composite.append(dict(path=str(f),line=n,expression=cls,source=line.strip()))
source=obj(TIP,'crates/rustuml-parser/src/parse/sequence.rs').decode();tails=[]
for n,line in enumerate(source.splitlines(),1):
 if ('try_' in line and '(rest' in line) or 'parse_inline_alias(' in line:tails.append(dict(line=n,text=line.strip()))
report=dict(reviewer='independent 2b056ca1 / gpt-6-astra high / Codex T3',baseline=BASE,evidence_tip=TIP,production=PROD,java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',account_order=facts,citations=citations,citation_failures=[c for c in citations if not c['exists']],production_changed=prod_changed,java_composite_classes=composite,tail_parser_locations=tails)
(C/'source-audit.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({k:report[k] for k in ['citation_failures','java_composite_classes','tail_parser_locations']},indent=2))
