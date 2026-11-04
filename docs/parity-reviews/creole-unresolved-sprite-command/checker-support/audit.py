# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,subprocess,hashlib
from pathlib import Path
R=Path.cwd();S=Path('/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-checker');B='0f00130a26b2f300996d0bd427f9e70b4ef10e30';C='246ff6396b538843143ee9d2cc9a569b67f614dd';A='b160f41e2076b98805d4e3c9262a46030394e67c';I='78f06098b74a81d24efcc922b3116d03ad496132'
sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*a):return subprocess.check_output(['git',*a],cwd=R)
old=json.loads((R/'docs/parity-reviews/creole-unresolved-sprite-command/maker-support/historical-files.json').read_bytes());rows=[]
for x in old:
 data=(R/x['path']).read_bytes();rows.append(dict(path=x['path'],sha256=sha(data),matches_maker=sha(data)==x['sha256'],matches_base=data==git('show',B+':'+x['path'])))
(S/'historical-audit.json').write_text(json.dumps(rows,indent=2)+'\n')
files=git('diff','--name-only',B,C).decode().splitlines();chron=[]
for rev in [B,A,I,C]:
 chron.append(dict(revision=rev,parents=git('show','-s','--format=%P',rev).decode().strip(),message=git('show','-s','--format=%B',rev).decode(),changes=git('diff-tree','--no-commit-id','--name-only','-r',rev).decode().splitlines()))
java=Path('/Users/marcelo/work/github.com/plantuml/plantuml');prefix='src/main/java/net/sourceforge/plantuml/'
paths=['klimt/creole/command/CommandCreoleSprite.java','klimt/creole/command/Splitter.java','klimt/sprite/SpriteUtils.java','klimt/creole/legacy/StripeSimple.java','klimt/creole/command/CommandCreoleStyle.java','klimt/creole/command/CommandCreoleSizeChange.java','klimt/creole/command/CommandCreoleColorChange.java','klimt/creole/Parser.java','regex/Pattern2.java','skin/SkinParam.java','klimt/creole/legacy/AtomText.java','klimt/creole/SheetBlock1.java','classdiagram/command/CommandCreateClass.java']
jrows=[]
for name in paths:
 p=prefix+name;b=(java/p).read_bytes();pinned=subprocess.check_output(['git','show','71806a23780b04a5ccde2f8ceb5121edad5eb711:'+p],cwd=java);jrows.append(dict(path=p,sha256=sha(b),matches_pinned=b==pinned))
summary=dict(candidate=C,base=B,chronology=chron,historical_file_count=len(rows),all_historical_unchanged=all(x['matches_maker'] and x['matches_base'] for x in rows),production_diff=[f for f in files if f.startswith('crates/')],java_sources=jrows,corpus_gitlink=git('ls-tree','HEAD','test-diagrams/golden').decode(),hydrated_corpus_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=R/'test-diagrams/golden',text=True).strip(),golden_puml_count=len(list((R/'test-diagrams/golden').rglob('*.puml'))),guarded_changes=[f for f in files if any(n in f for n in ['baseline','compare.rs','harness.rs','bullseye.yaml'])])
(S/'audit-summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(len(rows),summary['all_historical_unchanged'],summary['production_diff'],summary['golden_puml_count'])
