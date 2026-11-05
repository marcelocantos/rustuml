# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import os,subprocess,time,json,hashlib,concurrent.futures
from pathlib import Path
S=Path(__file__).parent

def build(phase):
 cwd=S/('source-'+phase);env=dict(os.environ,CARGO_TARGET_DIR=str(S/('target-'+phase)),CARGO_BUILD_JOBS='4');cmd=['cargo','build','-p','rustuml','--release','-j','4'];start=time.time()
 with (S/'logs'/('build-'+phase+'.log')).open('wb') as f:p=subprocess.run(cmd,cwd=cwd,env=env,stdout=f,stderr=subprocess.STDOUT)
 result=dict(phase=phase,cwd=str(cwd),argv=cmd,env_overrides={k:env[k] for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS']},exit=p.returncode,start=start,end=time.time())
 if p.returncode==0:
  import shutil
  binary=S/(phase+'-rustuml');shutil.copy2(S/('target-'+phase)/'release/rustuml',binary);result['binary_sha256']=hashlib.sha256(binary.read_bytes()).hexdigest()
 (S/('build-'+phase+'.json')).write_text(json.dumps(result,indent=2)+'\n');print(result,flush=True)
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:list(pool.map(build,['before','after']))
