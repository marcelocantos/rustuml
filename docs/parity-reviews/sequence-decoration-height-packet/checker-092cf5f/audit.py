# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections, hashlib, json, subprocess, tarfile
from pathlib import Path
ROOT=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-607d50f7'); CACHE=Path(__file__).parent
BEFORE='efc2e922bbd093cf13d827501f5e53aa850a9e68'; AFTER='092cf5f68464625c8b3771e069b42f44f43fc0c9'
def git(*args): return subprocess.check_output(['git',*args],cwd=ROOT)
def sha(b): return hashlib.sha256(b).hexdigest()
def tree(rev,*paths):
    rows=git('ls-tree','-r',rev,*paths).decode().splitlines()
    return {r.split('\t',1)[1]: r.split('\t',1)[0].split()[2] for r in rows}
def load(p): return json.loads((ROOT/p).read_text())
old=tree(BEFORE,'docs/parity-reviews','test-diagrams/perturbations')
new=tree(AFTER,'docs/parity-reviews','test-diagrams/perturbations')
changed=[p for p,v in old.items() if new.get(p)!=v]; assert not changed,changed
original='fdf1154eb3a17ebb041fffb0c18710a38210f56b'
imported=git('diff-tree','--no-commit-id','--name-only','-r',original).decode().splitlines()
ot=tree(original)
assert all(new.get(p)==ot[p] for p in imported)
packet=ROOT/'docs/parity-reviews/sequence-decoration-height-packet/maker-results'
rows=load(packet/'all-inputs.json')
inventory=[]
for r in rows:
    s=(ROOT/r['source']).read_bytes(); g=(ROOT/r['golden']).read_bytes()
    inventory.append(dict(r,source_sha256=sha(s),source_bytes=len(s),golden_sha256=sha(g),golden_bytes=len(g)))
assert len(rows)==1966 and len({r['source'] for r in rows})==1966
assert len({r['source_sha256'] for r in inventory})==1918
prior=load('docs/parity-reviews/sequence-corrective-b9/maker-results/all-inputs.json')
checker=load('docs/parity-reviews/sequence-corrective-b9/checker-ef020/pairs.json')
fresh=[r for r in checker if r['fresh']]; copies=[r for r in checker if not r['fresh']]
assert len(prior)==1901 and len(fresh)==37 and len(copies)==2
assert [r['source'] for r in rows[:1901]]==[r['source'] for r in prior]
assert {r['source'] for r in rows[1901:1938]}=={r['source'] for r in fresh}
assert not {r['source'] for r in copies}&{r['source'] for r in rows}
partitions=[inventory[:1901],inventory[1901:1938],inventory[1938:1962],inventory[1962:]]
hashsets=[{r['source_sha256'] for r in p} for p in partitions]
assert all(not hashsets[i]&hashsets[j] for i in range(4) for j in range(i))
ownership=git('diff','--name-only',BEFORE,AFTER,'--','crates').decode().splitlines()
assert len(ownership)==4
comparator=git('diff',BEFORE,AFTER,'--','crates/rustuml-oracle'); assert not comparator
ascii_diff=git('diff','--unified=0',BEFORE,AFTER,'--','crates/rustuml-render/src/ascii.rs').decode()
added=[l[1:] for l in ascii_diff.splitlines() if l.startswith('+') and not l.startswith('+++')]
removed=[l for l in ascii_diff.splitlines() if l.startswith('-') and not l.startswith('---')]
assert len(added)==2 and all(l.strip()=='target_circle: false,' for l in added) and not removed
chron=[]
for a in load(packet/'implementation-map.json')['accounts']:
    ret=subprocess.run(['git','merge-base','--is-ancestor',a['account_commit'],a['implementation_commit']],cwd=ROOT).returncode
    assert ret==0
    chron.append(dict(a,account_precedes_implementation=True,implementation_message=git('show','-s','--format=%B',a['implementation_commit']).decode()))
maker=load(packet/'binaries.json'); bindings=[]
for p,h in maker['after']['source_files'].items():
    actual=sha(git('show',AFTER+':'+p)); assert actual==h
    bindings.append(dict(path=p,sha256=actual,match=True))
assert not git('diff',maker['before']['original']['revision'],BEFORE,'--','crates')
captured={}
with tarfile.open(CACHE/'maker-captures.tar.gz','r:gz') as tar:
    for n in ['validation350e776a/final-binaries.json','validation026e5eeb/final-binaries.json','main-regression/measurements.json']:
        captured[n]=json.load(tar.extractfile(n))
for name,rev in [('validation350e776a/final-binaries.json','350e776a2cd376dd2171dc79b61010da8f01fc2e'),('validation026e5eeb/final-binaries.json','026e5eeb2f3521ec3a2f299b30d52a4cac51f05c')]:
    cap=captured[name]
    cap['independent_committed_hash_binding']={p:dict(capture=h,committed=sha(git('show',rev+':'+p)),equal=h==sha(git('show',rev+':'+p))) for p,h in cap['source_files'].items()}
record=dict(candidate=AFTER,before=BEFORE,historical_paths=len(old),historical_blob_map=old,changed_historical_paths=changed,import_original=original,import_commit='eaedee419da43b38fb91d8848cea39d325c55d1c',imported_paths=len(imported),imported_byte_identical=True,cohort_entries=len(rows),unique_paths=len({r['source'] for r in rows}),unique_source_bytes=len(hashsets[0]|hashsets[1]|hashsets[2]|hashsets[3]),duplicate_entries=48,cohorts=dict(collections.Counter(r['cohort'] for r in rows)),partition_sizes=list(map(len,partitions)),partition_hash_sets_disjoint=True,registered_copies_not_added=[r['source'] for r in copies],owned_files=ownership,comparator_unchanged=True,comparator_sha256=sha((ROOT/'crates/rustuml-oracle/src/compare.rs').read_bytes()),adapter_sha256=sha((ROOT/'docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs').read_bytes()),measure_sha256=sha((ROOT/'docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py').read_bytes()),ascii_diff=ascii_diff,chronology=chron,maker_final_binding=bindings,maker_intermediate_metadata=captured)
(CACHE/'audit.json').write_text(json.dumps(record,indent=2)+'\n')
(CACHE/'frozen-inventory.json').write_text(json.dumps(inventory,indent=2)+'\n')
print('historical',len(old),'imported',len(imported),'cohorts',record['cohorts'])
print('dirty/intermediate bindings',[(k,v.get('revision'),v.get('independent_committed_hash_binding')) for k,v in captured.items() if 'binaries' in k])
