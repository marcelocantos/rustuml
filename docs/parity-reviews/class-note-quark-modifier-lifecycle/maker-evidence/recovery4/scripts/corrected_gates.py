# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,os,subprocess,time
from pathlib import Path
s=Path(__file__).resolve().parent
# Deny local JVM launch even if a future test starts using the dormant oracle
# runner. The marker is retained so an attempted launch is visible, not hidden.
guard=s/'no-local-jvm';guard.mkdir(exist_ok=True)
for name in ['java','javac']:
 p=guard/name;p.write_text('#!/bin/sh\nprintf "%s\\n" "LOCAL_JVM_LAUNCH_DISABLED" >&2\nexit 126\n');p.chmod(0o700)
env=dict(os.environ,CARGO_TARGET_DIR=str(s/'target'),CARGO_BUILD_JOBS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
env['JAVA_TOOL_OPTIONS']=(env.get('JAVA_TOOL_OPTIONS','')+' -Djava.awt.headless=true').strip();env['PATH']=str(guard)+os.pathsep+env['PATH']
checks=[('fmt',['cargo','fmt','--check']),('units',['cargo','test','--workspace','--release','--lib','--no-fail-fast']),('cli-errors',['cargo','test','--workspace','--release','--test','pipe_mode','--test','error_paths','--test','doc_claims','--no-fail-fast']),('no_oracle',['cargo','test','--release','--test','golden_no_oracle','--','--nocapture']),('assisted',['cargo','test','--release','--test','golden_pairs','--','--nocapture']),('guards',['cargo','test','--release','--test','harness_guards','--','--nocapture']),('clippy',['cargo','clippy','--workspace','--release','--','-D','warnings'])]
records=[]
for name,cmd in checks:
 start=time.time()
 with (s/('corrected-'+name+'.log')).open('w') as f:p=subprocess.run(cmd,env=env,stdout=f,stderr=subprocess.STDOUT)
 records.append(dict(name=name,command=cmd,exit=p.returncode,seconds=time.time()-start,log='corrected-'+name+'.log',cwd=str(Path.cwd()),revision=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),environment={k:env[k] for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS','PLANTUML_STDLIB','RUSTUML_DEBUG','JAVA_TOOL_OPTIONS']},local_jvm_guard=str(guard)))
 (s/'corrected-gates.json').write_text(json.dumps(records,indent=2)+'\n');print(name,p.returncode,flush=True)
