# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import os,subprocess,time,json,tarfile,io,shutil
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();D=S/'source-after'
# Evidence-only overlay gives the exact supplied stable registration (1860).
data=subprocess.check_output(['git','archive','bdfeba117c7af6a527fc66b0acda6d8273fa6d74','docs','test-diagrams','bullseye.yaml'])
with tarfile.open(fileobj=io.BytesIO(data)) as t:t.extractall(D,filter='data')
env=dict(os.environ,CARGO_TARGET_DIR=str(S/'target-after'),CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',RUST_TEST_THREADS='4',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
cmds=[('library',['cargo','test','--workspace','--lib','--release']),('cli',['cargo','test','-p','rustuml','--test','pipe_mode','--release']),('focused',['cargo','test','-p','rustuml-render','--release','--lib','creole::'])]+[(n,['cargo','test','-p','rustuml-oracle','--release','--test',t,'--','--nocapture']) for n,t in [('main-product','golden_no_oracle'),('main-assisted','golden_pairs'),('standing','parity_perturbations'),('guards','harness_guards')]]+[('clippy',['cargo','clippy','--workspace','--all-targets','--','-D','warnings']),('format',['cargo','fmt','--all','--','--check']),('touched-format',['rustfmt','--edition','2024','--check','--config','skip_children=true','crates/rustuml-render/src/creole.rs'])]
rows=[]
for name,cmd in cmds:
 start=time.time()
 with (S/'logs'/f'{name}.log').open('wb') as f:q=subprocess.run(cmd,cwd=D,env=env,stdout=f,stderr=subprocess.STDOUT)
 rows.append(dict(name=name,argv=cmd,cwd=str(D),env_overrides={k:env[k] for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS','RAYON_NUM_THREADS','RUST_TEST_THREADS','RUSTUML_DEBUG','PLANTUML_STDLIB']},exit=q.returncode,seconds=time.time()-start));(S/'gates.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,q.returncode,flush=True)
 for filename in ['no_oracle_failure_names.txt','golden_failure_names.txt']:
  p=D/'test-diagrams'/filename
  if p.exists():shutil.copy2(p,S/filename)
