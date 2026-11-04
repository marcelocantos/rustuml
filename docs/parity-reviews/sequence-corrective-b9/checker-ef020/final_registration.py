# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,json,os,pathlib,subprocess
c=pathlib.Path(__file__).parent;r=pathlib.Path('/Users/marcelo/.t3/worktrees/rustuml/t3-ef020aa1');d=r/'docs/parity-reviews/sequence-corrective-b9/checker-ef020'
env=dict(os.environ,CARGO_TARGET_DIR=str(c/'after-target'),CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',RUST_TEST_THREADS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
argv=['cargo','test','--locked','--release','--no-fail-fast','-p','rustuml-oracle','--test','parity_perturbations','--test','harness_guards','--','--nocapture']
with (c/'final-registration.log').open('w') as f:p=subprocess.run(argv,cwd=r,env=env,stdout=f,stderr=subprocess.STDOUT)
commands=json.loads((d/'commands.json').read_text());commands.append(dict(name='final-registration',argv=argv,cwd=str(r),environment={k:env[k] for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS','RAYON_NUM_THREADS','RUST_TEST_THREADS','PLANTUML_STDLIB']},exit=p.returncode,reason='New evidence pairs and reviews registered; no broad main/unit rerun.'))
(d/'commands.json').write_text(json.dumps(commands,indent=2)+'\n');(d/'final-registration.log.gz').write_bytes(gzip.compress((c/'final-registration.log').read_bytes(),mtime=0));print('Final registration exit',p.returncode)
