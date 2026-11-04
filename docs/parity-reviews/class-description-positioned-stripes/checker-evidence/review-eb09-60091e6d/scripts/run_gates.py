# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import os,subprocess,time,json,hashlib
from pathlib import Path
S=Path(__file__).resolve().parent
assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()=='eb09cecbc90f5690c4a09bf49118f2feff8c1c12'
assert subprocess.check_output(['git','-C','test-diagrams/golden','rev-parse','HEAD'],text=True).strip()=='9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a'
assert len(list(Path('test-diagrams/golden').rglob('*.puml')))==12550
G=S/'no-jvm';G.mkdir(exist_ok=True)
for name in ['java','javac']:
 p=G/name
 if not p.exists():p.symlink_to('/usr/bin/false')
env=dict(os.environ,CARGO_TARGET_DIR=str(S/'target-after'),CARGO_BUILD_JOBS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
env['PATH']=str(G)+os.pathsep+env['PATH']
commands=[('build',['cargo','build','--release','-p','rustuml']),('fmt',['cargo','fmt','--check']),('units',['cargo','test','--workspace','--release','--lib','--no-fail-fast']),('cli',['cargo','test','--workspace','--release','--test','pipe_mode','--test','error_paths','--test','doc_claims','--no-fail-fast']),('product',['cargo','test','--release','--test','golden_no_oracle','--','--nocapture']),('assisted',['cargo','test','--release','--test','golden_pairs','--','--nocapture']),('guards',['cargo','test','--release','--test','harness_guards','--','--nocapture']),('clippy',['cargo','clippy','--workspace','--release','--','-D','warnings'])]
records=[]
for name,cmd in commands:
 start=time.time()
 with (S/(name+'.log')).open('wb') as f:p=subprocess.run(cmd,env=env,stdout=f,stderr=subprocess.STDOUT)
 records.append(dict(name=name,command=cmd,exit=p.returncode,seconds=time.time()-start,cwd=os.getcwd(),revision=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),environment={k:env[k] for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS','PLANTUML_STDLIB','RUSTUML_DEBUG','PATH']},log=name+'.log'))
 (S/'gates.json').write_text(json.dumps(records,indent=2)+'\n');print(name,p.returncode,flush=True)
 if name=='build' and p.returncode:break
