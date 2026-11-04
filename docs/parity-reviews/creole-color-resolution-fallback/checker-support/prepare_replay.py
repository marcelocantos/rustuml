# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import pathlib,json,shutil,hashlib,re
S=pathlib.Path(__file__).parent;M=S/'maker';R=pathlib.Path('/Users/marcelo/.t3/worktrees/rustuml/t3-ae47bffd');sha=lambda b:hashlib.sha256(b).hexdigest();mapping={}
for p in sorted(M.glob('*-membership.json')):
 if 'original' in p.name:continue
 rows=json.loads(p.read_bytes())
 for row in rows:
  for field in ['source_capture','golden_capture','reference_source_capture']:
   if field not in row:continue
   orig=pathlib.Path(row[field]);local=M/orig.relative_to('/Users/marcelo/Library/Caches/rustuml-creole-fallback-maker');assert local.is_file(),local
   if field!='reference_source_capture':assert sha(local.read_bytes())==row[field.replace('_capture','_sha256')]
   row[field]=str(local)
  old=row['cwd']
  if old not in mapping:
   if '/.t3/worktrees/rustuml/' in old:
    new=re.sub(r'/Users/marcelo/\.t3/worktrees/rustuml/t3-[^/]+',str(S/'src-after'),old)
   else:
    dest=S/'cwd-resources'/sha(old.encode())[:12]
    shutil.copytree(old,dest);new=str(dest)
   assert pathlib.Path(new).is_dir(),new;mapping[old]=new
  row['original_cwd']=old;row['cwd']=mapping[old]
 (S/p.name).write_text(json.dumps(rows,indent=2)+'\n')
(S/'cwd-mapping.json').write_text(json.dumps(mapping,indent=2)+'\n')
for name in ['replay.py','run_replay.py']:
 text=(M/name).read_text().replace('/Users/marcelo/.t3/worktrees/rustuml/t3-a295df45',str(S/'src-after'))
 (S/name).write_text(text)
print('frozen 2050 inputs; remapped cwd contracts',len(mapping))
