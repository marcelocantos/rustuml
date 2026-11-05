# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import subprocess,json,hashlib,tarfile,re
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();M=R/'docs/parity-reviews/creole-parameter-candidate-grammar/maker-support';J=Path('/Users/marcelo/work/github.com/plantuml/plantuml');pin='71806a23780b04a5ccde2f8ceb5121edad5eb711';base='2608469dace1f680a7e90c0895e44e657f8aaf7f';after='8ca73318f584dc29d7d8ab1c91f25a21718ca713'
sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*args):return subprocess.check_output(['git',*args])
def dump(name,v):(S/name).write_text(json.dumps(v,indent=2)+'\n')
archive=json.loads((M/'archive-files.json').read_text());bad=[]
for x in archive:
 p=S/'maker'/x['path'];b=p.read_bytes()
 if sha(b)!=x['sha256'] or len(b)!=x['bytes']:bad.append(x['path'])
dump('archive-check.json',dict(sha256=sha((M/'measurements.tar.gz').read_bytes()),members=len(archive),mismatches=bad));assert not bad
# Compare all historical blobs using git's object hash; the bytes are read independently.
raw=git('ls-tree','-r','-z',base,'--','docs/parity-reviews','test-diagrams/perturbations');rows=[]
for entry in raw.split(b'\0'):
 if not entry:continue
 header,path=entry.split(b'\t',1);p=path.decode();mode,kind,oid=header.decode().split();b=(R/p).read_bytes();actual=hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest();rows.append(dict(path=p,git_blob=oid,sha256=sha(b),equal=oid==actual))
dump('historical-base-check.json',dict(base=base,files=len(rows),all_equal=all(x['equal'] for x in rows),inventory=rows))
hist=json.loads((M/'historical-integrity.json').read_text());imports=[]
for imp in hist['imports']:
 checks=[]
 for p in imp['paths']:
  checks.append((R/p).read_bytes()==git('show',imp['revision']+':'+p))
 imports.append(dict(revision=imp['revision'],files=len(checks),all_equal=all(checks)))
dump('historical-import-check.json',dict(old_files=hist['old_files'],imports=imports))
js=[]
for x in json.loads((M/'java-source-provenance.json').read_text())['files']:
 p=Path(x['path']);b=p.read_bytes();f=str(p.relative_to(J));pinned=subprocess.check_output(['git','-C',str(J),'show',pin+':'+f]);js.append(dict(path=f,sha256=sha(b),matches_pin=b==pinned,matches_citation_hash=sha(b)==x['sha256']))
dump('java-source-check.json',dict(pin=pin,files=js));assert all(x['matches_pin'] and x['matches_citation_hash'] for x in js)
prod=git('diff','--name-only',base,after,'--','crates','vendor','Cargo.toml','Cargo.lock').decode().splitlines();assert prod==['crates/rustuml-render/src/creole.rs']
before=git('show',base+':crates/rustuml-render/src/creole.rs');final=git('show',after+':crates/rustuml-render/src/creole.rs')
ancestry=[]
for a,d in [('c14bea51a7e5797d1ba4f0ce9282f2fde378ad3b','3d92268c4aae5f8bfd3a0e271758b6a8e5442e47'),('5652ec397eab87fb9449beefb7e2368179ae1919',after)]:ancestry.append(dict(account=a,draft=d,precedes=subprocess.run(['git','merge-base','--is-ancestor',a,d]).returncode==0))
patches=[git('show','--format=','--binary',r) for r in ['05683a971037e7e9eced8c78d1e11dc198f81cca','4e2918c608c9845751c937088dd1a68631657316']]
dump('scope-check.json',dict(production_files=prod,legacy_prefix_byte_equal=before.split(b'fn walk_segments')[0]==final.split(b'fn walk_segments')[0],public_signatures_equal=re.findall(rb'^pub .*',before,re.M)==re.findall(rb'^pub .*',final,re.M),sprite_grammar_equal=before.split(b'fn is_sprite_tag')[1].split(b'fn push_segment')[0]==final.split(b'fn is_sprite_tag')[1].split(b'fn push_segment')[0],account_ancestry=ancestry,import_patch_byte_equal=patches[0]==patches[1],golden_pin=git('-C','test-diagrams/golden','rev-parse','HEAD').decode().strip()))
print('archive',len(archive),'base',len(rows),'imports',imports,'java',len(js),'scope',prod)
