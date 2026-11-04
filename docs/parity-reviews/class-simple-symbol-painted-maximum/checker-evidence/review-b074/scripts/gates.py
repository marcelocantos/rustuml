# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,os,subprocess,time
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-review-b074');E=S/'evidence';env=dict(os.environ,CARGO_TARGET_DIR=str(S/'target-after'),CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',RUST_TEST_THREADS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
for var in ['HARNESS_WRITE_BASELINES','GOLDEN_FILTER']:env.pop(var,None)
commands=[('format',['cargo','fmt','--check']),('libs',['cargo','test','--workspace','--release','--lib','--no-fail-fast']),('cli',['cargo','test','--workspace','--release','--test','pipe_mode','--test','error_paths','--test','doc_claims','--no-fail-fast']),('product',['cargo','test','--release','--test','golden_no_oracle','--','--nocapture']),('assisted',['cargo','test','--release','--test','golden_pairs','--','--nocapture']),('guards',['cargo','test','--release','--test','harness_guards','--','--nocapture']),('clippy',['cargo','clippy','--workspace','--release','--','-D','warnings'])]
records=[]
for name,cmd in commands:
    start=time.time(); rev=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip();assert rev=='b074d2082cb9ed2b9460f79a32b170e31cc1ab5e'
    with (E/f'{name}.log').open('wb') as out:p=subprocess.run(cmd,env=env,stdout=out,stderr=subprocess.STDOUT)
    records.append(dict(name=name,command=cmd,exit=p.returncode,revision=rev,cwd=str(Path.cwd()),started_unix=start,seconds=time.time()-start,environment={k:env[k] for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS','RAYON_NUM_THREADS','RUST_TEST_THREADS','PLANTUML_STDLIB','RUSTUML_DEBUG']}));(E/'gates.json').write_text(json.dumps(records,indent=2)+'\n');print(name,p.returncode,flush=True)
