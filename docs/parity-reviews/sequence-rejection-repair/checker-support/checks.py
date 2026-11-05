# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Run final candidate checks once, preserving failures and complete logs."""
import json
import os
from pathlib import Path
import subprocess
import sys
ROOT=Path(__file__).resolve().parents[4]
OUT=Path('/Users/marcelo/Library/Caches/rustuml-sequence-review-d382aba6')
env=dict(os.environ,CARGO_TARGET_DIR=str(OUT/'target'),CARGO_BUILD_JOBS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
assert not any(k in env for k in ['GOLDEN_FILTER','HARNESS_WRITE_BASELINES'])
checks=[
 ('main-tiers',['cargo','test','--release','--no-fail-fast','-p','rustuml-oracle','--test','golden_no_oracle','--test','golden_pairs','--','--nocapture']),
 ('units',['cargo','test','--release','--lib','-p','rustuml-parser','-p','rustuml-render','--no-fail-fast']),
 ('cli',['cargo','test','--release','-p','rustuml','--test','pipe_mode']),
 ('format-workspace',['cargo','fmt','--all','--','--check']),
 ('format-touched',['rustfmt','--check','--edition','2024','crates/rustuml-parser/src/diagram/sequence.rs','crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs']),
 ('clippy',['cargo','clippy','--release','-p','rustuml-parser','-p','rustuml-render','--','-D','warnings']),
 ('clippy-render',['cargo','clippy','--release','-p','rustuml-render','--no-deps','--','-D','warnings']),
]
if sys.argv[1:] == ['evidence']:
 checks=[('evidence-gates',['cargo','test','--release','--no-fail-fast','-p','rustuml-oracle','--test','harness_guards','--test','parity_perturbations','--','--nocapture'])]
results=[]
for name,command in checks:
 path=OUT/'logs'/(name+'.log')
 with path.open('xb') as log:
  result=subprocess.run(command,cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
 results.append(dict(name=name,command=command,exit=result.returncode,cwd=str(ROOT),candidate='0fb4e14b7227eb768d268128d42fa5f6d5a5b1af',log=str(path)))
 (OUT/('checks-evidence.json' if sys.argv[1:] else 'checks.json')).write_text(json.dumps(results,indent=2)+'\n')
 print(name,result.returncode,flush=True)
