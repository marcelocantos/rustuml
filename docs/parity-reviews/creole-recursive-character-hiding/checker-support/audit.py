# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,subprocess,hashlib,re
from pathlib import Path
R=Path.cwd();S=Path(__file__).parent;M=S/'maker';J=Path('/Users/marcelo/work/github.com/plantuml/plantuml')
base='a3afe487a66ae26c73e2f1853d6c26fc2d228875';tip='2608469dace1f680a7e90c0895e44e657f8aaf7f';prod='a6e1718c4eb693aec99c3a107fd654afb8a4e4e3';account='5d039c21c9374ee91904fe4da430c22ff6e2c4f3';jrev='71806a23780b04a5ccde2f8ceb5121edad5eb711'
sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*a,cwd=R):return subprocess.check_output(['git',*a],cwd=cwd)
def dump(n,v):(S/n).write_text(json.dumps(v,indent=2)+'\n')
rows=[]
for line in git('ls-tree','-r',base,'--','docs/parity-reviews','test-diagrams/perturbations').decode().splitlines():
 meta,path=line.split('\t');blob=meta.split()[2];data=(R/path).read_bytes();actual=hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest();rows.append(dict(path=path,git_blob=blob,sha256=sha(data),equal=actual==blob))
assert len(rows)==10347 and all(x['equal'] for x in rows);dump('base-preservation.json',dict(revision=base,files=len(rows),inventory=rows))
hist=json.loads((M/'historical-integrity.json').read_bytes());imports=[]
for rec in hist['imports']:
 diffs=[]
 for p in rec['paths']:
  if (R/p).read_bytes()!=git('show',rec['revision']+':'+p):diffs.append(p)
 imports.append(dict(revision=rec['revision'],files=len(rec['paths']),differences=diffs))
 assert not diffs
# The preserved prior inventories have their original content; verify their listed bytes too.
old=json.loads((R/'docs/parity-reviews/creole-style-candidate-eligibility/maker-support/historical-integrity.json').read_bytes())
# Find the original 9731-file inventory rather than substituting the maker summary.
invs=[]
for p in (R/'docs/parity-reviews').rglob('*inventory*.json'):
 try:d=json.loads(p.read_bytes())
 except (ValueError,OSError):continue
 if isinstance(d,list) and len(d)>=9000:invs.append((p,d))
for p,d in invs:
 checked=0;errors=[]
 for x in d:
  if not isinstance(x,dict) or 'path' not in x or 'sha256' not in x:continue
  q=Path(x['path']);q=q if q.is_absolute() else R/q
  if not q.exists() or sha(q.read_bytes())!=x['sha256']:errors.append(x['path'])
  checked+=1
 imports.append(dict(inventory=str(p.relative_to(R)),files=checked,differences=errors));assert not errors
archive_inv=json.loads((R/'docs/parity-reviews/creole-recursive-character-hiding/maker-support/archive-files.json').read_bytes())
assert all(sha((M/x['path']).read_bytes())==x['sha256'] and (M/x['path']).stat().st_size==x['bytes'] for x in archive_inv)
dump('integrity-summary.json',dict(base_files=len(rows),imports=imports,maker_archive_sha256=sha((R/'docs/parity-reviews/creole-recursive-character-hiding/maker-support/measurements.tar.gz').read_bytes()),maker_archive_members=len(archive_inv),maker_archive_members_exact=True))
java=[]
for row in json.loads((M/'java-source-provenance.json').read_bytes())['files']:
 p=Path(row['path']);b=p.read_bytes();pinned=git('show',jrev+':'+str(p.relative_to(J)),cwd=J);java.append(dict(path=str(p.relative_to(J)),sha256=sha(b),equal_to_pin=b==pinned,equal_to_cited_hash=sha(b)==row['sha256']))
assert all(x['equal_to_pin'] and x['equal_to_cited_hash'] for x in java)
dump('java-source-audit.json',dict(revision=jrev,local_head=git('rev-parse','HEAD',cwd=J).decode().strip(),files=java))
production_paths=git('diff','--name-only',base,prod,'--','crates','vendor','Cargo.toml','Cargo.lock').decode().splitlines();assert production_paths==['crates/rustuml-render/src/creole.rs']
b=git('show',base+':crates/rustuml-render/src/creole.rs');a=git('show',prod+':crates/rustuml-render/src/creole.rs');public=lambda t:re.findall(rb'^pub (?:fn|struct|enum|type|const).*',t,re.M)
assert b[:b.index(b'pub struct Style')]==a[:a.index(b'pub struct Style')];assert public(b)==public(a)
assert not git('diff',prod,tip,'--','crates','vendor','Cargo.toml','Cargo.lock')
ancestry=[]
for earlier,later in [(base,'f75103e509a5c5cd61341d78421a436f7103603f'),('f75103e509a5c5cd61341d78421a436f7103603f',account),(account,prod),(prod,tip)]:
 rc=subprocess.run(['git','merge-base','--is-ancestor',earlier,later]).returncode;assert rc==0;ancestry.append(dict(earlier=earlier,later=later,exit=rc))
import_diff=git('diff','5ec48be56c4bce12ec646c29ed8e6d4be5e8518b^','5ec48be56c4bce12ec646c29ed8e6d4be5e8518b','--binary');copy_diff=git('diff','f75103e509a5c5cd61341d78421a436f7103603f^','f75103e509a5c5cd61341d78421a436f7103603f','--binary');assert import_diff==copy_diff
dump('scope-chronology.json',dict(base=base,tip=tip,production=prod,account=account,production_paths=production_paths,legacy_prefix_unchanged=True,public_signatures_unchanged=True,prod_tip_code_equal=True,import_patch_byte_equal=True,ancestry=ancestry,commit_records=git('log','--format=fuller','--reverse',base+'..'+tip).decode(),limitation='Git proves account-before-committed-draft ancestry, not absence of unrecorded pre-account drafts.'))
print('Audit verified',len(rows),'base files;',len(archive_inv),'archive members;',len(java),'pinned Java files;',imports)
