# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,subprocess,hashlib,pathlib
S=pathlib.Path(__file__).parent;R=pathlib.Path.cwd();J=pathlib.Path('/Users/marcelo/work/github.com/plantuml/plantuml')
B='246ff6396b538843143ee9d2cc9a569b67f614dd';C='a84856637adcd8e8d4e78fd2ee00658c9c612feb';I='59dc60604792260c342f6e6ab96829f10b0a59f5';V='71806a23780b04a5ccde2f8ceb5121edad5eb711'
sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*a,cwd=R):return subprocess.check_output(['git',*a],cwd=cwd)
def dump(n,v):(S/n).write_text(json.dumps(v,indent=2)+'\n')
old=json.loads((R/'docs/parity-reviews/creole-unresolved-sprite-command/maker-support/historical-files.json').read_bytes());rows=[]
for x in old:
 b=(R/x['path']).read_bytes();rows.append(dict(path=x['path'],sha256=sha(b),expected_sha256=x['sha256'],matches=sha(b)==x['sha256']))
assert len(rows)==9731 and all(x['matches'] for x in rows)
imported=git('diff-tree','--no-commit-id','--name-only','-r','83469a4832cf20b8e2e57514841e5c46adf63a6e').decode().splitlines();ir=[]
for p in imported:
 b=(R/p).read_bytes();original=git('show','83469a4832cf20b8e2e57514841e5c46adf63a6e:'+p);ir.append(dict(path=p,sha256=sha(b),original_sha256=sha(original),matches=b==original))
assert len(ir)==118 and all(x['matches'] for x in ir)
dump('historical-audit.json',dict(old=rows,imported=ir))
jrows=[]
for x in json.loads((R/'docs/parity-reviews/creole-unresolved-sprite-command/corrective-maker-support/java-source-provenance.json').read_bytes())['files']:
 p=pathlib.Path(x['path']);b=p.read_bytes();pin=git('show',V+':'+str(p.relative_to(J)),cwd=J);jrows.append(dict(path=str(p),sha256=sha(b),matches_pinned=b==pin,matches_maker=sha(b)==x['sha256'],locations=x['locations']))
assert all(x['matches_pinned'] and x['matches_maker'] for x in jrows)
chron=[]
for rev in [B,'6b0ca33e88127a44bad1e564544db7d746816d6d','ee02618929097265e3e48508bdb078242d2f50d4','e25a4a20b4df62f9347afb292fda5940941e8bc7',I,C]:
 chron.append(dict(revision=rev,parents=git('show','-s','--format=%P',rev).decode().strip(),message=git('show','-s','--format=%B',rev).decode(),changes=git('diff-tree','--no-commit-id','--name-only','-r',rev).decode().splitlines()))
files=git('diff','--name-only',B,C).decode().splitlines()
dump('audit-summary.json',dict(base=B,candidate=C,implementation=I,chronology=chron,old_files_exact=len(rows),imported_files_exact=len(ir),production_changes=[p for p in files if p.startswith('crates/')],guarded_changes=[p for p in files if any(k in p for k in ['baseline','compare.rs','harness.rs','bullseye.yaml'])],java_revision=git('rev-parse','HEAD',cwd=J).decode().strip(),java_sources=jrows,submodule_expected='9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a',submodule_actual=git('rev-parse','HEAD',cwd=R/'test-diagrams/golden').decode().strip(),golden_puml_count=len(list((R/'test-diagrams/golden').rglob('*.puml')))))
print('9731 old + 118 imported exact; pinned Java sources exact')
