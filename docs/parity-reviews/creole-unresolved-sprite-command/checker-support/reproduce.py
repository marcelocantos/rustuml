# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import importlib.util,json,hashlib,subprocess,concurrent.futures,re,shutil,os
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-checker');M=Path('/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-maker');R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-9a8dea60')
sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
spec=importlib.util.spec_from_file_location('replay',R/'docs/parity-reviews/creole-unresolved-sprite-command/maker-support/replay.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);m.R=R;m.S=S
# Build unchanged diagnostic source against this exact candidate's libraries.
deps=S/'target-after/release/deps';builds=[]
for stem,names in [('probe',['parser','oracle']),('source_probe',['parser']),('standing_slice',['parser','oracle','render'])]:
 p=S/(stem+'.rs');p.write_bytes((M/(stem+'.rs')).read_bytes());argv=['rustc','--edition=2024',str(p),'-L','dependency='+str(deps)]
 for n in names:
  lib=next(deps.glob('librustuml_'+n+'-*.rlib'));argv+=['--extern','rustuml_'+n+'='+str(lib)]
 out=S/({'source_probe':'source-probe','standing_slice':'standing-slice'}.get(stem,stem));argv+=['-o',str(out)]
 q=subprocess.run(argv,capture_output=True);(S/'logs'/(stem+'-build.stdout')).write_bytes(q.stdout);(S/'logs'/(stem+'-build.stderr')).write_bytes(q.stderr)
 builds.append(dict(argv=argv,exit=q.returncode,source_sha256=sha(p.read_bytes()),binary_sha256=sha(out.read_bytes()) if q.returncode==0 else None));dump(S/'diagnostic-builds.json',builds);assert q.returncode==0
rows=json.loads((M/'membership.json').read_bytes())+json.loads((M/'checker8-membership.json').read_bytes());assert len(rows)==1730
for row in rows:
 row['cwd']=re.sub(r'/Users/marcelo/\.t3/worktrees/rustuml/t3-[^/]+',str(R),row['cwd'])
 for key in ['source_capture','golden_capture','reference_source_capture']:
  if key not in row:continue
  p=Path(row[key]);raw=p.read_bytes();field=key.removesuffix('_capture')+'_sha256'
  if field in row:assert sha(raw)==row[field]
  dest=S/'retained-inputs'/row['id']/p.name;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(raw);row[key]=str(dest);row[field]=sha(raw)
dump(S/'retained-membership.json',rows)
fresh=[]
for i,x in enumerate(json.loads((M/'fresh-after-results.json').read_bytes())):
 p=Path(x['row']['source']);gold=M/'fresh-reference'/p.stem/'java.svg.stdout';reference=M/'fresh-reference'/p.stem/'java.source.stdout'
 fresh.append(dict(id='maker18/'+p.stem,suite='maker18',index=i,membership=dict(x['row'],reference_valid=x['reference_valid']),cwd=str(p.parent),source_capture=str(p),golden_capture=str(gold),reference_source_capture=str(reference)))
sprites=json.loads((M/'sprites-membership.json').read_bytes())
for x in sprites:
 for k in ['cwd','source_capture','golden_capture']:x[k]=re.sub(r'/Users/marcelo/\.t3/worktrees/rustuml/t3-[^/]+',str(R),x[k])
for name,cohort in [('retained',rows),('maker18',fresh),('sprites50',sprites)]:
 dump(S/(name+'-membership.json'),cohort)
 for phase in ['before','after']:
  with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:results=list(pool.map(lambda r:m.measure(r,phase),cohort))
  dump(S/(name+'-'+phase+'.json'),results);print(name,phase,len(results),sum(x['reference_valid'] for x in results),sum(x['svg_match'] for x in results),flush=True)
