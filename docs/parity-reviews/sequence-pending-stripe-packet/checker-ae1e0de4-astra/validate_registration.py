# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,gzip,hashlib,subprocess,tarfile,re,collections
from pathlib import Path
C=Path(__file__).parent;R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-78c595f4');D=R/'docs/parity-reviews/sequence-pending-stripe-packet/checker-ae1e0de4-astra'
sha=lambda b:hashlib.sha256(b).hexdigest()
def read(p):return json.loads(gzip.decompress(p.read_bytes()) if p.suffix=='.gz' else p.read_bytes())
def write(p,x):p.write_text(json.dumps(x,indent=2)+'\n')
# Extend the count-level inventory to the separate10; original86 remains captured unmodified.
inventory=read(C/'semantic-inventory.json.gz')
for row in read(C/'followup10/measurements.json.gz'):
 rec=dict(cohort='followup10',source=row['source'],java=row['reference_inventory'],versions={})
 for rev in ['before','after']:
  result=row['results'][rev];ast=(C/'followup10'/result['ast']['output']).read_text();events=ast.split('        events: [',1)[-1].split('        autonumber:',1)[0] if '        events: [' in ast else ''
  rec['versions'][rev]=dict(ast_exit=result['ast']['exit'],event_counts=dict(collections.Counter(re.findall(r'^            ([A-Za-z_]+)(?:\(|,)',events,re.M))),svg_exit=result['cli']['exit'],inventory=result['cli']['inventory'])
 inventory.append(rec)
(D/'semantic-inventory.json.gz').write_bytes(gzip.compress(json.dumps(inventory).encode(),mtime=0))
preserved=read(D/'preservation-and-chronology.json');protected={**preserved['prior_blobs'],**preserved['import_blobs']}
for path,blob in protected.items():
 b=(R/path).read_bytes();assert hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()==blob,path
index=read(D/'review-index.json');names=set();review_hashes={}
for x in index:
 p=R/x['review'];v=read(p);review_hashes[x['review']]=sha(p.read_bytes())
 for field in ['schema_version','mechanism','reviewer','account_commit','implementation_commit','java_revision','commands','perturbations','findings','verdict']:assert v[field]
 for e in v['perturbations']:
  a=R/e['source'];b=R/e['golden'];assert a.with_suffix('.svg')==b;assert sha(a.read_bytes())==e['source_sha256'];assert sha(b.read_bytes())==e['reference_sha256'];assert e['result']==('pass' if e['comparison_exit']==0 else 'fail');names.add(e['source'])
 if v['verdict']=='accepted':assert all(e['result']=='pass' for e in v['perturbations'])
assert len(names)==96
archive_hashes=read(D/'archive-files.json');n=0
with tarfile.open(D/'execution-captures.tar.gz') as tar:
 for member in tar.getmembers():
  if member.isfile():assert sha(tar.extractfile(member).read())==archive_hashes[member.name];n+=1
assert n==len(archive_hashes)
assert subprocess.check_output(['git','diff','--name-only'],cwd=R)==b''
for a,b in [('711dbca7c5e29592395f0927881aabfc2517b217','a4c3c35794ae1d7117d709134ed22c7191db9db8'),('ae1e0de4d39840dd58074d89d40aa97304c6ca9f','653b1ed7a49784e6e6205f3aebf69a16278f8d78')]:
 assert subprocess.check_output(['git','rev-parse',a+':crates'],cwd=R)==subprocess.check_output(['git','rev-parse',b+':crates'],cwd=R)
# The immutable hydrated corpus must match every blob at the pinned golden revision.
golden=Path('/Users/marcelo/work/github.com/marcelocantos/rustuml/test-diagrams/golden');entries=subprocess.check_output(['git','ls-tree','-r','-z','9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a'],cwd=golden).split(b'\0');gn=0
for line in entries:
 if not line:continue
 meta,path=line.split(b'\t',1);b=(R/'test-diagrams/golden'/path.decode()).read_bytes();assert hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()==meta.decode().split()[2];gn+=1
# Count exact content integrity, not executor claims, without rerunning stable product gates.
write(D/'registration-validation.json',dict(protected_paths=len(protected),baseline_paths=len(preserved['prior_blobs']),imported_paths=len(preserved['import_blobs']),changed_protected_paths=[],fresh_pairs=len(names),reviews=len(index),review_sha256=review_hashes,archive_members=n,source_crate_trees_match_exact_production=True,golden_revision='9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a',golden_files_verified=gn,production_diff='none',compiler=subprocess.check_output(['rustc','--version'],text=True).strip(),cargo_lock_sha256=sha((R/'Cargo.lock').read_bytes()),scope='Evidence schema/blob/hash registration validation; no repeated broad test run'))
write(D/'artifact-sha256.json',{str(p.relative_to(D)):sha(p.read_bytes()) for p in sorted(D.rglob('*')) if p.is_file() and p.name!='artifact-sha256.json'})
print('verified',len(protected),'protected paths;',len(names),'pairs;',n,'archive members;',gn,'golden blobs; production diff empty')
