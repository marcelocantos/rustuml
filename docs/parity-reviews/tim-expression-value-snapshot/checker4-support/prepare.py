# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,subprocess,shutil,tarfile
from pathlib import Path
ROOT=Path.cwd();S=Path('/Users/marcelo/Library/Caches/rustuml-review-cb14b5f3');P=ROOT/'docs/parity-reviews/tim-typed-values-and-binding/checker3-support'
OLD=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-bf706fe0');REV='463fc9bd1c60b4fc0fae49230031e5ba2a693fc7'
def git(*args):return subprocess.check_output(['git',*args],cwd=ROOT)
def sha(b):return hashlib.sha256(b).hexdigest()
assert not git('diff','--name-only','579b9b1173fef9dd3d10afca05b6d688b9bbd2c2','7bb210e9','--','crates','Cargo.toml','Cargo.lock').strip()
restored=[];verified=[]
tracked=set(git('ls-tree','-r','--name-only','HEAD','--','test-diagrams/perturbations').decode().splitlines())
for raw in git('ls-tree','-r','--full-tree',REV,'--','test-diagrams/perturbations').decode().splitlines():
    meta,path=raw.split('\t');mode,typ,oid=meta.split();assert typ=='blob' and mode in {'100644','100755'}
    data=(OLD/path).read_bytes();assert hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()==oid
    dest=ROOT/path
    if dest.exists():assert dest.read_bytes()==data,path
    else:dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(data);dest.chmod(int(mode[-3:],8))
    if path not in tracked:restored.append(path)
    verified.append(dict(path=path,blob=oid,sha256=sha(data)))
rows=[];before=[]
for suite,file in [('frozen','frozen-inventory.json'),('fresh','fresh-manifest.json'),('retained','retained-manifest.json'),('supplement','supplement-manifest.json')]:
    data=json.loads((P/file).read_text());previous=json.loads((P/f'after-{suite}.json').read_text());assert len(data)==len(previous)
    previous={r['source']:r for r in previous}
    for row in data:
        source=ROOT/row['source'];golden=source.with_suffix('.svg')
        if 'source_sha256' in row:assert row['source_sha256']==sha(source.read_bytes())
        if 'golden_sha256' in row:assert row['golden_sha256']==sha(golden.read_bytes())
        rows.append(dict(row,cohort=suite,source_sha256=sha(source.read_bytes()),golden_sha256=sha(golden.read_bytes())))
        before.append(dict(previous[row['source']],cohort=suite))
assert len(rows)==1516 and len({r['source'] for r in rows})==1516
(S/'collateral-manifest.json').write_text(json.dumps(rows,indent=2)+'\n')
(S/'baseline-results.json').write_text(json.dumps(before,indent=2)+'\n')
(S/'preservation-initial.json').write_text(json.dumps(dict(exact_candidate=git('rev-parse','HEAD').decode().strip(),baseline_production='7bb210e9f7578705638c8f3b7c3b43d8b4b906c3',preserved_measurement_production='579b9b1173fef9dd3d10afca05b6d688b9bbd2c2',production_diff_empty=True,baseline_source_evidence_revision=REV,restored=restored,verified=verified),indent=2)+'\n')
with tarfile.open(P/'measurement-artifacts.tar.gz') as t:
    for m in t:
        if m.isfile() and m.name.startswith('reference/'):
            path=Path(m.name);assert not path.is_absolute() and '..' not in path.parts
            dest=S/'baseline-capture'/path;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(t.extractfile(m).read())
shutil.copyfile(S/'target/release/rustuml',S/'candidate-rustuml')
lib=S/'target/release/deps'
cmd=['rustc','--edition=2024',str(P/'probe.rs'),'-L',f'dependency={lib}','--extern',f'rustuml_parser={next(lib.glob("librustuml_parser-*.rlib"))}','--extern',f'rustuml_oracle={next(lib.glob("librustuml_oracle-*.rlib"))}','-o',str(S/'probe')]
subprocess.run(cmd,check=True)
(S/'probe-build.json').write_text(json.dumps(dict(command=cmd,binary_sha256=sha((S/'probe').read_bytes()),candidate_sha256=sha((S/'candidate-rustuml').read_bytes())),indent=2)+'\n')
print('restored',len(restored),'sources',sum(p.endswith('.puml') for p in restored),'verified',len(verified),'cohort',len(rows))
