# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
import subprocess,os,json,time
base=Path('/tmp/rustuml-cff3');env=dict(os.environ,CARGO_TARGET_DIR='/tmp/rustuml-cff-target',CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',JAVA_TOOL_OPTIONS=(os.environ.get('JAVA_TOOL_OPTIONS','')+' -Djava.awt.headless=true').strip())
assert not env.get('HARNESS_WRITE_BASELINES') and not env.get('GOLDEN_FILTER')
commands=[('fmt',['cargo','fmt','--all','--','--check']),('units-final',['cargo','test','--workspace','--lib','--release']),('cli-parser',['cargo','test','--release','--test','pipe_mode','--test','error_paths']),('golden-no-oracle',['cargo','test','--test','golden_no_oracle','--release','--','--nocapture']),('golden-assisted',['cargo','test','--test','golden_pairs','--release','--','--nocapture']),('guards',['cargo','test','--test','harness_guards','--release','--','--nocapture']),('perturbations',['cargo','test','--test','parity_perturbations','--release','--','--nocapture']),('clippy-final',['cargo','clippy','--workspace','--all-targets','--','-D','warnings'])]
records=[]
for name,cmd in commands:
 start=time.time()
 with (base/(name+'.log')).open('wb') as log: r=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
 records.append(dict(name=name,command=cmd,exit=r.returncode,seconds=time.time()-start,log=name+'.log'))
 (base/'gates.json').write_text(json.dumps(dict(revision=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),cwd=str(Path.cwd()),environment={k:env[k] for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS','RAYON_NUM_THREADS','PLANTUML_STDLIB','RUSTUML_DEBUG','JAVA_TOOL_OPTIONS']},java_policy='No Java launched: selected tests use existing pairs or local Rust CLI; runner.run_preproc is not called.',commands=records),indent=2)+'\n')
 print(name,r.returncode,round(time.time()-start,2),flush=True)
