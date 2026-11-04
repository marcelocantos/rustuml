# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,os,subprocess,sys,time
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-review-cb14b5f3');R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-cb14b5f3')
env=dict(os.environ,CARGO_TARGET_DIR=str(S/'target'),CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
for k in ['HARNESS_WRITE_BASELINES','GOLDEN_FILTER']:env.pop(k,None)
commands={'basic':[
('parser',['cargo','test','-p','rustuml-parser','--lib']),('cli',['cargo','test','-p','rustuml','--test','pipe_mode']),('format',['rustfmt','--edition','2024','--check','crates/rustuml-parser/src/preprocess/mod.rs']),('clippy',['cargo','clippy','--workspace','--','-D','warnings'])],
'main': [('main-product',['cargo','test','-p','rustuml-oracle','--release','--test','golden_no_oracle','--','--nocapture']),('main-assisted',['cargo','test','-p','rustuml-oracle','--release','--test','golden_pairs','--','--nocapture'])],
'final': [('perturbations',['cargo','test','-p','rustuml-oracle','--release','--test','parity_perturbations','--','--nocapture']),('guards',['cargo','test','-p','rustuml-oracle','--release','--test','harness_guards','--','--nocapture'])]}
results=[]
for name,cmd in commands[sys.argv[1]]:
 t=time.time()
 with (S/'logs'/f'{name}.log').open('wb') as f:p=subprocess.run(cmd,cwd=R,env=env,stdout=f,stderr=subprocess.STDOUT)
 results.append(dict(name=name,command=cmd,exit=p.returncode,seconds=time.time()-t))
 (S/f'validation-{sys.argv[1]}.json').write_text(json.dumps(results,indent=2)+'\n')
 print(name,p.returncode,flush=True)
