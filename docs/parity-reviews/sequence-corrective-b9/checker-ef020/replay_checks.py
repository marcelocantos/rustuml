# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import pathlib,os,subprocess,json
r=pathlib.Path.cwd();c=pathlib.Path(__file__).parent;records=[]
env=dict(os.environ,CARGO_TARGET_DIR=str(c/'after-target'),CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',RUST_TEST_THREADS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
for k in ['HARNESS_WRITE_BASELINES','RUSTUML_ORACLE']:env.pop(k,None)
def run(name,args):
 with (c/(name+'.log')).open('wb') as f:p=subprocess.run(args,cwd=r,env=env,stdout=f,stderr=subprocess.STDOUT)
 records.append(dict(name=name,argv=args,cwd=str(r),exit=p.returncode,environment={k:env[k] for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS','RAYON_NUM_THREADS','RUST_TEST_THREADS','PLANTUML_STDLIB']}));(c/'checks.json').write_text(json.dumps(records,indent=2));print(name,p.returncode,flush=True)
for label in ['before','after']:
 run('cohort-'+label,['uv','run','--no-project','python','docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py','docs/parity-reviews/sequence-corrective-b9/maker-results/all-inputs.json',str(c/(label+'-bin')),str(c/('cohort-'+label))])
for name,args in [
 ('units',['cargo','test','--locked','--release','--lib','-p','rustuml-parser','-p','rustuml-render','--no-fail-fast']),
 ('cli',['cargo','test','--locked','--release','-p','rustuml','--test','pipe_mode']),
 ('main',['cargo','test','--locked','--release','--no-fail-fast','-p','rustuml-oracle','--test','golden_no_oracle','--test','golden_pairs','--test','parity_perturbations','--test','harness_guards','--','--nocapture']),
 ('fmt-owned',['rustfmt','--check','--edition','2024','crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs']),
 ('fmt-workspace',['cargo','fmt','--all','--','--check']),
 ('clippy',['cargo','clippy','--locked','-p','rustuml-parser','-p','rustuml-render','--','-D','warnings']),
 ('clippy-render',['cargo','clippy','--locked','-p','rustuml-render','--no-deps','--','-D','warnings'])]:run(name,args)
