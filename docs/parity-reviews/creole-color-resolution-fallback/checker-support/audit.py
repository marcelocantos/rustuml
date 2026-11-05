# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import subprocess,json,hashlib,pathlib,re
S=pathlib.Path(__file__).parent;R=pathlib.Path('/Users/marcelo/.t3/worktrees/rustuml/t3-ae47bffd');M=S/'maker';J=pathlib.Path('/Users/marcelo/work/github.com/plantuml/plantuml');base='bdfeba117c7af6a527fc66b0acda6d8273fa6d74';tip='b8905e296b8865e53f3c573abaebf4adb632d751';prod='1ffb930b7b56decb15208121db5e70d252ed3c1b';account='f977806cef5a90a868175e8e707a89d4d791d29d';jrev='71806a23780b04a5ccde2f8ceb5121edad5eb711';sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*args,cwd=R):return subprocess.check_output(['git',*args],cwd=cwd)
def dump(n,x):(S/n).write_text(json.dumps(x,indent=2)+'\n')
assert git('diff','--name-only',base,prod,'--','crates').decode().splitlines()==['crates/rustuml-render/src/creole.rs']
assert subprocess.run(['git','merge-base','--is-ancestor',account,prod],cwd=R).returncode==0
assert git('rev-parse',prod+'^').decode().strip()==account
prodmsg=git('show','-s','--format=%B',prod).decode()
before=git('show',base+':crates/rustuml-render/src/creole.rs');after=git('show',prod+':crates/rustuml-render/src/creole.rs')
assert before[:before.index(b'pub struct Style')]==after[:after.index(b'pub struct Style')]
assert before[before.index(b'pub struct Style'):before.index(b'pub fn parse_segments')]==after[after.index(b'pub struct Style'):after.index(b'pub fn parse_segments')]
assert git('diff','--name-only',prod,tip,'--','crates')==b''
paths=git('ls-tree','-r','--name-only',base,'--','docs/parity-reviews','test-diagrams/perturbations').decode().splitlines();rows=[]
with subprocess.Popen(['git','cat-file','--batch'],cwd=R,stdin=subprocess.PIPE,stdout=subprocess.PIPE) as proc:
 for p in paths:
  proc.stdin.write((base+':'+p+'\n').encode());proc.stdin.flush();h=proc.stdout.readline().split();b=proc.stdout.read(int(h[2]));assert proc.stdout.read(1)==b'\n';assert b==(R/p).read_bytes(),p;rows.append(dict(path=p,sha256=sha(b)))
 proc.stdin.close();assert proc.wait()==0
assert len(rows)==10721;dump('base-preservation.json',rows)
imports=json.loads((M/'historical-integrity.json').read_bytes())['imports'];imports.append(json.loads((M/'rejection-import.json').read_bytes()))
for record in imports:
 rev=record.get('revision',record.get('original'))
 for p in record['paths']:assert git('show',rev+':'+p)==(R/p).read_bytes(),p
 print('import verified',rev,len(record['paths']),flush=True)
# Historic inventory enumerated upstream, independent of maker summary.
inv=json.loads((R/'docs/parity-reviews/creole-parameter-candidate-grammar/maker-support/historical-integrity.json').read_bytes())
print('historical fields',inv.keys(),flush=True)
jrows=[]
for x in json.loads((M/'java-source-provenance.json').read_bytes())['files']:
 p=pathlib.Path(x['path']);b=git('show',jrev+':'+str(p.relative_to(J)),cwd=J);assert b==p.read_bytes();assert sha(b)==x['sha256'];jrows.append(x)
dump('java-source-audit.json',dict(revision=jrev,head=git('rev-parse','HEAD',cwd=J).decode().strip(),files=jrows))
archive=json.loads((R/'docs/parity-reviews/creole-color-resolution-fallback/maker-support/archive-files.json').read_bytes());print('archive type',type(archive).__name__,str(archive)[:200],flush=True)
dump('audit-summary.json',dict(production_files=['crates/rustuml-render/src/creole.rs'],account_precedes_draft=True,production_parent=account,implementation_message=prodmsg,legacy_tspans_unchanged=True,style_public_api_unchanged=True,base_files=len(rows),imports=[dict(revision=x.get('revision',x.get('original')),files=len(x['paths'])) for x in imports],java_files=len(jrows),golden_revision=git('rev-parse','HEAD',cwd=R/'test-diagrams/golden').decode().strip(),fixture_shaped_production_logic=False,new_constants_provenance='HColorSet.parseSimpleColor widths 1/3/6/8; HColors.WHITE; ColorTrieNode seven Archimate names and absence of RebeccaPurple; AtomImg.create monospaced(14) blackBlueTrue. No numerical layout tuning.'))
