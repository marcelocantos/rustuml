# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,gzip,hashlib,json,re,subprocess,tarfile
from pathlib import Path
R=Path.cwd(); C=Path(__file__).parent
M=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-8eddc9af/docs/parity-reviews/sequence-consumer-contract-packet/maker-results')
TIP='9784e412146e3ece634d88ea0a1bddd6b42a82b6';BASE='bce6bea405d8b5efd55136f4768117115e7f634f';IMP='f3913f3cb7094d8aedd5618f1ba27830a2804713';LOCAL='b286fe630c92ee33bfea4bd1b6aa7307ce6e1a6a';ACCOUNT='0b512262aaf4bcf0c0ab144b9f28c0719f329e4c';J='71806a23780b04a5ccde2f8ceb5121edad5eb711'
sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*args):return subprocess.check_output(['git',*args],cwd=R)
def tree(rev):return {l.split('\t',1)[1]:l.split()[2] for l in git('ls-tree','-r',rev).decode().splitlines()}
b,t,imp,local=map(tree,[BASE,TIP,IMP,LOCAL]); old={p:h for p,h in b.items() if p.startswith(('docs/parity-reviews/','test-diagrams/perturbations/'))}
imported=git('diff-tree','--no-commit-id','--name-only','-r',IMP).decode().splitlines()
res=dict(prior_paths=len(old),prior_changed=[p for p,h in old.items() if t.get(p)!=h],imported_paths=len(imported),import_changed_at_import=[p for p in imported if imp[p]!=local.get(p)],import_changed_at_tip=[p for p in imported if imp[p]!=t.get(p)],existing_paths_modified=[p for p,h in b.items() if p in t and t[p]!=h])
res['production_baseline_equivalence']=git('diff','--name-only','61e6aeed6',BASE,'--','crates','Cargo.toml','Cargo.lock').decode().splitlines()
res['production_tip_equivalence']=git('diff','--name-only','d491d4bda',TIP,'--','crates','Cargo.toml','Cargo.lock').decode().splitlines()
implementations=['df9078e2e162752328ffa52ebb28f227e56bc8f6','f4b3db075f61e2a5444777a305ab9bb64fd3685d','47fb7bd3a27d62ed80cdca112721be7ca61fc2e6','d491d4bda47faf90d4751484d45aa1387fb1b290']
res['account_ancestry']=[dict(implementation=v,account_is_ancestor=subprocess.run(['git','merge-base','--is-ancestor',ACCOUNT,v],cwd=R).returncode==0,message=git('show','-s','--format=%B',v).decode()) for v in implementations]
java=Path('/Users/marcelo/work/github.com/plantuml/plantuml');refs=json.loads((M/'regex-source-map.json').read_text())['records'];records=[]
for r in refs:
 data=subprocess.check_output(['git','show',J+':'+r['java_path']],cwd=java);records.append(dict(r,pinned_sha256=sha(data),working_sha256=sha((java/r['java_path']).read_bytes()),matches=r['sha256']==sha(data)==sha((java/r['java_path']).read_bytes())))
res['regex_source_map']=records
# All account Java classes, including those omitted from the regex map.
classes=set()
for p in git('diff-tree','--no-commit-id','--name-only','-r',ACCOUNT).decode().splitlines():
 a=json.loads(git('show',ACCOUNT+':'+p));classes.update('src/main/java/net/sourceforge/plantuml/'+s.split('#')[0].replace('.','/')+'.java' for s in a['java_locations'])
res['account_java_sources']=[]
for path in sorted(classes):
 data=subprocess.check_output(['git','show',J+':'+path],cwd=java);res['account_java_sources'].append(dict(path=path,sha256=sha(data),working_matches_pinned=(java/path).read_bytes()==data))
res['artifact_integrity']=[dict(path=x['path'],matches=sha((M/x['path']).read_bytes())==x['sha256']) for x in json.loads((M/'artifact-integrity.json').read_text())['files']]
with tarfile.open(M/'captures.tar.gz') as archive:
 def data(n):return archive.extractfile(n).read()
 raw=json.loads(data('before2519/measurements.json'));combined=json.loads(data('before2549/measurements.json'))
 normalize=lambda r:{k:v for k,v in r.items() if k!='cohort'}
 res['measurement_label_recovery']=dict(original_rows=len(raw),combined_rows=len(combined),nonmetadata_row_changes=[i for i,(a,b) in enumerate(zip(raw,combined)) if normalize(a)!=normalize(b)],stdout_capture_changes=[n for n in archive.getnames() if n.startswith('before2519/') and n.endswith('.svg') and data(n)!=data(n.replace('before2519/','before2549/'))])
 original=json.loads(data('all2519.json'));labeled=json.loads(data('all2519-labeled.json'));res['measurement_label_recovery']['manifest_nonmetadata_changes']=[i for i,(a,b) in enumerate(zip(original,labeled)) if normalize(a)!=normalize(b)]
 res['measurement_label_recovery']['original_log']=data('before2519.log').decode()
 # Read all archive contents and preserve hashes without extracting executable contents.
 inventory=[dict(path=m.name,bytes=m.size,sha256=sha(archive.extractfile(m).read())) for m in archive.getmembers() if m.isfile()]
 (C/'maker-archive-inventory.json.gz').write_bytes(gzip.compress(json.dumps(inventory).encode(),mtime=0))
res['maker_inventory_read']={}
for filename in ['participant-kind-results.json.gz','semantic-inventory.json.gz','remaining-failures.json.gz']:
 obj=json.loads(gzip.decompress((M/filename).read_bytes()));res['maker_inventory_read'][filename]=dict(entries=len(obj),sha256=sha((M/filename).read_bytes()))
res['maker_kind_claims']={k:dict(entries=v['entries'],before=v['before']['match'],after=v['after']['match']) for k,v in json.loads(gzip.decompress((M/'participant-kind-results.json.gz').read_bytes())).items()}
# Duplicates count from bytes, not rows declared by the maker.
inputs=json.loads((M/'all-inputs.json').read_text());bs=[]
for item in inputs:
 path=Path(item['source']);path=R/path if not path.is_absolute() else path
 if not path.exists():path=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-8eddc9af')/item['source']
 bs.append(sha(path.read_bytes()))
fresh=json.loads((C/'reference-manifest.json').read_text())['cases']
res['cohort_counts']=dict(entries=len(bs),unique=len(set(bs)),duplicates=len(bs)-len(set(bs)),fresh=len(fresh),fresh_unique=len(set(x['source_sha256'] for x in fresh)),fresh_overlaps_old=sorted(set(bs)&{x['source_sha256'] for x in fresh}))
(C/'packet-audit.json').write_text(json.dumps(res,indent=2)+'\n')
print(json.dumps({k:v for k,v in res.items() if k not in ['artifact_integrity','account_java_sources','regex_source_map','account_ancestry']},indent=2))
