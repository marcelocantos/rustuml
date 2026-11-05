# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,os,subprocess,time
from pathlib import Path
S=Path(__file__).parent
assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()=='1703a2727fa364ee8004ba59a22b4c6934ee5d3d'
assert subprocess.check_output(['git','-C','test-diagrams/golden','rev-parse','HEAD'],text=True).strip()=='9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a'
env=dict(os.environ,CARGO_TARGET_DIR=str(S/'target-after'),CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',RUST_TEST_THREADS='4',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
cmds=[('parser',['cargo','test','-p','rustuml-parser','--lib','--release']),('cli',['cargo','test','-p','rustuml','--test','pipe_mode','--release'])]+[(n,['cargo','test','-p','rustuml-oracle','--release','--test',t,'--','--nocapture']) for n,t in [('main-product','golden_no_oracle'),('main-assisted','golden_pairs'),('standing','parity_perturbations'),('guards','harness_guards')]]+[('clippy',['cargo','clippy','-p','rustuml-parser','--','-D','warnings']),('format',['rustfmt','--edition','2024','--check','--config','skip_children=true','crates/rustuml-parser/src/preprocess/mod.rs']),('diff',['git','diff','--check'])]
rows=[]
for name,cmd in cmds:
 start=time.monotonic()
 with (S/'logs'/f'{name}.log').open('wb') as f:r=subprocess.run(cmd,env=env,stdout=f,stderr=subprocess.STDOUT)
 rows.append(dict(name=name,argv=cmd,cwd=str(Path.cwd()),exit=r.returncode,seconds=time.monotonic()-start));(S/'gates.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,r.returncode,flush=True)
