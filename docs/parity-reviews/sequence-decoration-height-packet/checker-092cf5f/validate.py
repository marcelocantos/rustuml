# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json, os, subprocess
from pathlib import Path
cache=Path(__file__).parent
root=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-607d50f7')
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()=='092cf5f68464625c8b3771e069b42f44f43fc0c9'
assert subprocess.check_output(['git','-C','test-diagrams/golden','rev-parse','HEAD'],cwd=root,text=True).strip()=='9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a'
env=dict(os.environ,CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',CARGO_TARGET_DIR=str(cache/'target-after'),PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
commands=[
 ('units',['cargo','test','--release','--lib','-p','rustuml-parser','-p','rustuml-render']),
 ('cli',['cargo','test','--release','-p','rustuml','--test','pipe_mode']),
 ('main-standing-guards',['cargo','test','--release','--no-fail-fast','-p','rustuml-oracle','--test','golden_no_oracle','--test','golden_pairs','--test','parity_perturbations','--test','harness_guards','--','--nocapture']),
 ('format-owned',['rustfmt','--check','--edition','2024','crates/rustuml-parser/src/diagram/sequence.rs','crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs','crates/rustuml-render/src/ascii.rs']),
 ('format-workspace',['cargo','fmt','--all','--','--check']),
 ('clippy',['cargo','clippy','-p','rustuml-parser','-p','rustuml-render','--','-D','warnings']),
 ('clippy-render',['cargo','clippy','-p','rustuml-render','--no-deps','--','-D','warnings']),
]
records=[]
for name,argv in commands:
    with (cache/(name+'.log')).open('wb') as log:
        p=subprocess.run(argv,cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT)
    records.append(dict(name=name,command=argv,cwd=str(root),exit=p.returncode,environment={k:env[k] for k in ['CARGO_BUILD_JOBS','RAYON_NUM_THREADS','CARGO_TARGET_DIR','PLANTUML_STDLIB','RUSTUML_DEBUG']}))
    (cache/'checks.json').write_text(json.dumps(records,indent=2)+'\n')
    print(name,p.returncode,flush=True)
