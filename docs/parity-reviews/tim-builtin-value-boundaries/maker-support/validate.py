# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Local Rust gates only. Existing golden data; no reference generation."""
import json,os,subprocess,time
from pathlib import Path
s=Path('/tmp/rustuml-tim5');env=dict(os.environ,CARGO_TARGET_DIR='/tmp/rustuml-tim5-target',CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',JAVA_TOOL_OPTIONS=(os.environ.get('JAVA_TOOL_OPTIONS','')+' -Djava.awt.headless=true').strip())
commands=[
 ('main-product',['cargo','test','-p','rustuml-oracle','--release','--test','golden_no_oracle','--','--nocapture']),
 ('main-assisted',['cargo','test','-p','rustuml-oracle','--release','--test','golden_pairs','--','--nocapture']),
 ('perturbations',['cargo','test','-p','rustuml-oracle','--release','--test','parity_perturbations','--','--nocapture']),
 ('guards',['cargo','test','-p','rustuml-oracle','--release','--test','harness_guards','--','--nocapture']),
 ('cli',['cargo','test','-p','rustuml','--test','pipe_mode']),
 ('clippy',['cargo','clippy','-p','rustuml-parser','--','-D','warnings']),
 ('format',['rustfmt','--edition','2024','--check','crates/rustuml-parser/src/preprocess/mod.rs']),
 ('diff',['git','diff','--check']),
]
results=[]
for name,cmd in commands:
 start=time.monotonic()
 with (s/(name+'.log')).open('wb') as log:r=subprocess.run(cmd,env=env,stdout=log,stderr=subprocess.STDOUT)
 results.append(dict(name=name,command=cmd,exit=r.returncode,seconds=time.monotonic()-start));(s/'validation.json').write_text(json.dumps(results,indent=2)+'\n');print(name,r.returncode,flush=True)
