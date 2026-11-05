# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,os,subprocess,time
from pathlib import Path
s=Path('/tmp/rustuml-tim3')
env=dict(os.environ,CARGO_TARGET_DIR='/tmp/rustuml-tim-repair-target',CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
env['JAVA_TOOL_OPTIONS']=(env.get('JAVA_TOOL_OPTIONS','')+' -Djava.awt.headless=true').strip()
for key in ['GOLDEN_FILTER','HARNESS_WRITE_BASELINES']:env.pop(key,None)
commands=[('fmt',['cargo','fmt','--check']),('clippy',['cargo','clippy','--workspace','--','-D','warnings']),('parser',['cargo','test','-p','rustuml-parser','--lib']),('cli',['cargo','test','-p','rustuml','--test','pipe_mode']),('tiers',['cargo','test','--workspace','--release','--no-fail-fast','--test','golden_no_oracle','--test','golden_pairs','--test','harness_guards','--test','doc_claims','--test','parity_perturbations','--','--nocapture'])]
results=[]
for name,cmd in commands:
 start=time.time()
 with (s/('final-'+name+'.log')).open('wb') as f:r=subprocess.run(cmd,env=env,stdout=f,stderr=subprocess.STDOUT)
 results.append(dict(check=name,command=cmd,exit=r.returncode,seconds=time.time()-start))
 (s/'final-checks.json').write_text(json.dumps(results,indent=2)+'\n')
 print(json.dumps(results[-1]),flush=True)
