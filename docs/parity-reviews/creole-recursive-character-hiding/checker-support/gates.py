# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,os,subprocess,time
from pathlib import Path
S=Path(__file__).parent
env=dict(os.environ,CARGO_TARGET_DIR=str(S/'target-after'),CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',RUST_TEST_THREADS='4',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
cmds=[('library',['cargo','test','--workspace','--lib','--release']),('cli',['cargo','test','-p','rustuml','--test','pipe_mode','--release'])]+[(n,['cargo','test','-p','rustuml-oracle','--release','--test',t,'--','--nocapture']) for n,t in [('main-product','golden_no_oracle'),('main-assisted','golden_pairs'),('standing','parity_perturbations'),('guards','harness_guards')]]+[('clippy',['cargo','clippy','--workspace','--all-targets','--','-D','warnings']),('format',['cargo','fmt','--all','--','--check']),('touched-format',['rustfmt','--edition','2024','--check','--config','skip_children=true','crates/rustuml-render/src/creole.rs']),('diff',['git','diff','--check'])]
rows=[]
for name,cmd in cmds:
 start=time.monotonic()
 with (S/'logs'/f'{name}.log').open('wb') as f:q=subprocess.run(cmd,env=env,stdout=f,stderr=subprocess.STDOUT)
 rows.append(dict(name=name,argv=cmd,cwd=str(Path.cwd()),env_overrides={k:env[k] for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS','RAYON_NUM_THREADS','RUST_TEST_THREADS','RUSTUML_DEBUG','PLANTUML_STDLIB']},exit=q.returncode,seconds=time.monotonic()-start));(S/'gates.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,q.returncode,flush=True)
