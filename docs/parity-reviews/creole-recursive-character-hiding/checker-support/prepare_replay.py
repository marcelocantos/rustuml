# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,shutil,re
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();M=S/'maker';sha=lambda b:hashlib.sha256(b).hexdigest()
cohorts=['retained','maker18','sprites50','checker38','fresh15','fresh26','new15','delimiters4','focused2','checker30','supplement8','fresh12'];mapping={}
shutil.copy2(M/'replay.py',S/'replay.py');shutil.copy2(M/'probe.rs',S/'probe.rs')
for cohort in cohorts:
 rows=json.loads((M/(cohort+'-membership.json')).read_bytes())
 for row in rows:
  for field in ['source_capture','golden_capture','reference_source_capture']:
   if field not in row:continue
   old=Path(row[field]);p=M/old.relative_to('/Users/marcelo/Library/Caches/rustuml-creole-hide-flush-maker');assert p.exists(),p
   assert sha(p.read_bytes())==row[field.replace('_capture','_sha256')]
   row[field]=str(p)
  old=row['cwd'];new=re.sub(r'/Users/marcelo/\.t3/worktrees/rustuml/t3-[^/]+',str(R),old)
  # Private cwd copy retains include resources; repo paths map into this immutable evidence tip.
  if old==new and old not in mapping:
   p=Path(old);assert p.is_dir(),old
   dest=S/'cwd-resources'/f'{len(mapping):03}-{p.parent.name}-{p.name}'
   shutil.copytree(p,dest,ignore=shutil.ignore_patterns('__pycache__'))
   mapping[old]=str(dest)
  elif old not in mapping:mapping[old]=new
  row['cwd']=mapping[old];assert Path(row['cwd']).is_dir()
 (S/(cohort+'-membership.json')).write_text(json.dumps(rows,indent=2)+'\n')
(S/'cwd-mapping.json').write_text(json.dumps(mapping,indent=2)+'\n')
s=(M/'run_replay.py').read_text();s=re.sub(r"R=Path\('[^']+'\)", 'R=Path('+repr(str(R))+')',s);(S/'run_replay.py').write_text(s)
print('Prepared',sum(len(json.loads((S/(c+'-membership.json')).read_bytes())) for c in cohorts),'observations',len(mapping),'cwd mappings')
