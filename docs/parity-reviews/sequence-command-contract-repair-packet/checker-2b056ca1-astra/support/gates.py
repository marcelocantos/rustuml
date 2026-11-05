# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,os,shutil,subprocess,sys
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();label=sys.argv[1];L=C/(label+'-gates');L.mkdir(exist_ok=False)
env=dict(os.environ,CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',CARGO_TARGET_DIR=str(C/(label+'-target')),PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
for k in ['HARNESS_WRITE_BASELINES','RUSTUML_ORACLE']:env.pop(k,None)
cmds=[('units',['cargo','test','--release','--lib','-p','rustuml-parser','-p','rustuml-render']),('cli',['cargo','test','--release','-p','rustuml','--test','pipe_mode']),('broad',['cargo','test','--release','--no-fail-fast','-p','rustuml-oracle','--test','golden_no_oracle','--test','golden_pairs','--test','parity_perturbations','--test','harness_guards','--','--nocapture'])]
if label=='after':cmds.extend([('fmt-owned',['rustfmt','--check','--edition','2024','--config','skip_children=true','crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs']),('fmt-workspace',['cargo','fmt','--all','--','--check']),('clippy',['cargo','clippy','-p','rustuml-parser','-p','rustuml-render','--','-D','warnings']),('clippy-render',['cargo','clippy','-p','rustuml-render','--no-deps','--','-D','warnings'])])
records=[]
for name,argv in cmds:
 with (L/(name+'.log')).open('wb') as f:p=subprocess.run(argv,cwd=R,env=env,stdout=f,stderr=subprocess.STDOUT)
 if name=='broad':
  for fn in ['no_oracle_failure_names.txt','golden_failure_names.txt']:shutil.copy2(R/'test-diagrams'/fn,L/fn)
 records.append(dict(name=name,argv=argv,exit=p.returncode,cwd=str(R),revision=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()))
 (L/'commands.json').write_text(json.dumps(records,indent=2)+'\n');print(label,name,p.returncode,flush=True)
