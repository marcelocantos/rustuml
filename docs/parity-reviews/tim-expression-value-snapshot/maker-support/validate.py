# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,os,subprocess
from pathlib import Path
s=Path('/tmp/rustuml-tim4');env=dict(os.environ,CARGO_TARGET_DIR='/tmp/rustuml-tim-repair-target',CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',JAVA_TOOL_OPTIONS=(os.environ.get('JAVA_TOOL_OPTIONS','')+' -Djava.awt.headless=true').strip())
commands=[
 ('oracle-final',['cargo','test','-p','rustuml-oracle','--release','--test','golden_no_oracle','--test','golden_pairs','--test','parity_perturbations','--test','harness_guards','--no-fail-fast','--','--nocapture']),
 ('cli-final',['cargo','test','-p','rustuml','--test','pipe_mode']),
 ('parser-final',['cargo','test','-p','rustuml-parser','--lib']),
 ('clippy-final',['cargo','clippy','-p','rustuml-parser','--','-D','warnings']),
 ('format-final',['rustfmt','--edition','2024','--check','crates/rustuml-parser/src/preprocess/mod.rs']),
 ('diff-final',['git','diff','--check']),
]
results=[]
for name,cmd in commands:
 with (s/(name+'.log')).open('wb') as log:r=subprocess.run(cmd,env=env,stdout=log,stderr=subprocess.STDOUT)
 results.append(dict(name=name,command=cmd,exit=r.returncode));(s/'validation.json').write_text(json.dumps(results,indent=2)+'\n');print(name,r.returncode,flush=True)
