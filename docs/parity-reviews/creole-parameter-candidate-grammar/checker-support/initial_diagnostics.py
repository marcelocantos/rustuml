# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import os,subprocess,json,time,shutil,hashlib
from pathlib import Path
S=Path(__file__).parent;rows=[]
for phase in ['initial','before']:
 D=S/('source-'+phase);env=dict(os.environ,CARGO_TARGET_DIR=str(S/('target-'+phase)),CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',RUST_TEST_THREADS='4',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
 for name,cmd in ([('build',['cargo','build','-p','rustuml','--release','-j','4'])] if phase=='initial' else [])+[(n,['cargo','test','-p','rustuml-oracle','--release','--test',test,'--','--nocapture']) for n,test in [('product','golden_no_oracle'),('assisted','golden_pairs')]]:
  start=time.time()
  with (S/'logs'/f'{phase}-{name}.log').open('wb') as f:q=subprocess.run(cmd,cwd=D,env=env,stdout=f,stderr=subprocess.STDOUT)
  r=dict(phase=phase,name=name,argv=cmd,cwd=str(D),exit=q.returncode,seconds=time.time()-start);rows.append(r)
  if name=='build' and q.returncode==0:
   binary=S/(phase+'-rustuml');shutil.copy2(S/('target-'+phase)/'release/rustuml',binary);r['binary_sha256']=hashlib.sha256(binary.read_bytes()).hexdigest()
  for filename in ['no_oracle_failure_names.txt','golden_failure_names.txt']:
   p=D/'test-diagrams'/filename
   if p.exists():shutil.copy2(p,S/(phase+'-'+filename))
  (S/'initial-diagnostics.json').write_text(json.dumps(rows,indent=2)+'\n');print(phase,name,q.returncode,flush=True)
