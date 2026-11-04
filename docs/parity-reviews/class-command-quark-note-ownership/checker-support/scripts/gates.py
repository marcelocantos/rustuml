# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json, os, subprocess, time
from pathlib import Path
s=Path('/tmp/rustuml-review-cbd')
env=dict(os.environ,CARGO_TARGET_DIR=str(s/'target'),CARGO_BUILD_JOBS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
checks=[('fmt',['cargo','fmt','--check']),('units',['cargo','test','--workspace','--release','--lib','--no-fail-fast']),('cli-errors',['cargo','test','--workspace','--release','--test','pipe_mode','--test','error_paths','--test','doc_claims','--no-fail-fast']),('no_oracle',['cargo','test','--release','--test','golden_no_oracle','--','--nocapture']),('assisted',['cargo','test','--release','--test','golden_pairs','--','--nocapture']),('clippy',['cargo','clippy','--workspace','--','-D','warnings'])]
records=[]
for name,cmd in checks:
    start=time.time()
    with (s/(name+'.log')).open('w') as f:
        p=subprocess.run(cmd,env=env,stdout=f,stderr=subprocess.STDOUT)
    records.append(dict(name=name,command=cmd,exit=p.returncode,seconds=time.time()-start,log=name+'.log',cwd=str(Path.cwd()),revision=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()))
    (s/'gates.json').write_text(json.dumps(records,indent=2)+'\n')
    print(name,p.returncode,flush=True)
