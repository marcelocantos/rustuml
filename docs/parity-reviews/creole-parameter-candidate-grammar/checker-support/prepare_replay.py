# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,re,shutil
from pathlib import Path
S=Path(__file__).parent;M=S/'maker';R=Path.cwd();sha=lambda b:hashlib.sha256(b).hexdigest();maps={}
for p in M.glob('*-membership.json'):
 if '-original-' in p.name:continue
 rows=json.loads(p.read_text())
 for row in rows:
  for field in ['source_capture','golden_capture','reference_source_capture']:
   if field not in row:continue
   q=Path(row[field].replace('/Users/marcelo/Library/Caches/rustuml-creole-parameter-maker',str(M)));assert q.exists(),q
   row[field]=str(q)
   if field!='reference_source_capture':assert sha(q.read_bytes())==row[field.replace('_capture','_sha256')]
  cwd=row['cwd']
  if cwd not in maps:
   if '/.t3/worktrees/rustuml/' in cwd:new=Path(re.sub(r'/Users/marcelo/\.t3/worktrees/rustuml/t3-[^/]+',str(R),cwd))
   elif cwd.startswith('/Users/marcelo/Library/Caches/rustuml-creole-parameter-maker'):new=Path(cwd.replace('/Users/marcelo/Library/Caches/rustuml-creole-parameter-maker',str(M)))
   else:
    new=S/'cwd-resources'/sha(cwd.encode())[:16];shutil.copytree(cwd,new,dirs_exist_ok=True,ignore=shutil.ignore_patterns('__pycache__'))
   assert new.is_dir(),new;maps[cwd]=str(new)
  row['cwd']=maps[cwd]
 (S/p.name).write_text(json.dumps(rows,indent=2)+'\n')
(S/'cwd-mapping.json').write_text(json.dumps(maps,indent=2)+'\n')
shutil.copy2(M/'replay.py',S/'replay.py');x=(M/'run_replay.py').read_text();x=re.sub(r"R=Path\('[^']+'\)",f'R=Path({str(R)!r})',x);(S/'run_replay.py').write_text(x)
print('prepared',len(maps),'cwd mappings')
