# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import pathlib,json,hashlib,shutil,subprocess
S=pathlib.Path(__file__).parent;M=S.parent/'rustuml-sprite-corrective-maker';R=pathlib.Path.cwd()
sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
for cohort in ['retained','maker18','sprites50','checker38','fresh15']:
 rows=json.loads((M/(cohort+'-membership.json')).read_bytes())
 shutil.copy2(M/(cohort+'-membership.json'),S/(cohort+'-original-membership.json'))
 for r in rows:
  for key in ['source_capture','golden_capture','reference_source_capture']:
   if key not in r:continue
   p=pathlib.Path(r[key]);b=p.read_bytes();h=sha(b);field=key.removesuffix('_capture')+'_sha256'
   if field in r:assert h==r[field],(r['id'],field)
   dest=S/'frozen'/r['id']/p.name;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(b);r['prior_'+key]=str(p);r[key]=str(dest);r[field]=h
 dump(S/(cohort+'-membership.json'),rows)
# Reuse the exact harness source but bind output and working repository to this reviewer.
shutil.copy2(M/'replay.py',S/'replay.py');shutil.copy2(M/'probe.rs',S/'probe.rs')
text=(M/'run_replay.py').read_text().replace("R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-a295df45')",'R=Path('+repr(str(R))+')')
(S/'run_replay.py').write_text(text)
print('frozen five cohorts',flush=True)
