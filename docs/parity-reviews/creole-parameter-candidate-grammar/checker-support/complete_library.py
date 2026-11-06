# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import subprocess,os,json,time
from pathlib import Path
S=Path(__file__).parent;D=S/'source-after';(D/'Makefile').write_bytes(subprocess.check_output(['git','show','8ca73318f584dc29d7d8ab1c91f25a21718ca713:Makefile']))
env=dict(os.environ,CARGO_TARGET_DIR=str(S/'target-after'),CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',RUST_TEST_THREADS='4',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib');rows=[]
for name,cmd in [('library-missing',['cargo','test','-p','rustuml-parser','-p','rustuml-render','--lib','--release']),('oracle-setup-recovery',['cargo','test','-p','rustuml-oracle','--lib','--release','harness::tests::make_gate_preserves_failure_and_runs_remaining_checks'])]:
 start=time.time()
 with (S/'logs'/f'{name}.log').open('wb') as f:q=subprocess.run(cmd,cwd=D,env=env,stdout=f,stderr=subprocess.STDOUT)
 rows.append(dict(name=name,argv=cmd,cwd=str(D),exit=q.returncode,seconds=time.time()-start));print(name,q.returncode,flush=True)
(S/'library-completion.json').write_text(json.dumps(dict(setup_failure='Initial source snapshot omitted Makefile; oracle harness test failed before parser/render ran. Preserved initial failure. Restored exact Makefile; ran only previously unrun crates and the single setup-failed oracle test. No passing job or broad golden suite repeated.',commands=rows),indent=2)+'\n')
