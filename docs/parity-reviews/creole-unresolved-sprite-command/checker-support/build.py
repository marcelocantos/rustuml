# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import subprocess, pathlib, os, time, json, hashlib, shutil
S=pathlib.Path('/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-checker')
R=pathlib.Path('/Users/marcelo/.t3/worktrees/rustuml/t3-9a8dea60')
branch='review/unresolved-sprite-246ff639-astra-20261104'
rows=[]
def run(name,cmd,env):
 t=time.monotonic()
 with (S/'logs'/f'{name}.log').open('wb') as f:p=subprocess.run(cmd,cwd=R,env=env,stdout=f,stderr=subprocess.STDOUT)
 row=dict(name=name,argv=cmd,cwd=str(R),revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=R,text=True).strip(),env_overrides={k:env[k] for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS'] if k in env},exit=p.returncode,seconds=time.monotonic()-t)
 rows.append(row);(S/'builds.json').write_text(json.dumps(rows,indent=2));print(name,p.returncode,flush=True)
 if p.returncode:raise RuntimeError(name)
try:
 subprocess.run(['git','switch','--detach','0f00130a26b2f300996d0bd427f9e70b4ef10e30'],cwd=R,check=True)
 for phase in ['before','after']:
  if phase=='after':subprocess.run(['git','switch',branch],cwd=R,check=True)
  env=dict(os.environ,CARGO_BUILD_JOBS='4',CARGO_TARGET_DIR=str(S/('target-'+phase)))
  run(phase+'-check',['cargo','check','--workspace'],env)
  run(phase+'-build',['cargo','build','-p','rustuml','--release'],env)
  shutil.copy2(S/('target-'+phase)/'release/rustuml',S/(phase+'-rustuml'))
  rows[-1]['binary_sha256']=hashlib.sha256((S/(phase+'-rustuml')).read_bytes()).hexdigest();(S/'builds.json').write_text(json.dumps(rows,indent=2))
finally:
 subprocess.run(['git','switch',branch],cwd=R,check=True)
