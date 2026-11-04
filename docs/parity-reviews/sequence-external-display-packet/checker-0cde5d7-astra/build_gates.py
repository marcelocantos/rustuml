# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,os,shutil,subprocess,time
from pathlib import Path
ROOT=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-5fc3056e')
CACHE=Path(__file__).resolve().parent
BRANCH='review/external-display-independent-5fc3056e'
revisions={'before':'653b1ed7a49784e6e6205f3aebf69a16278f8d78','after':'d714bb55d40ca18590d15a5ffd57c7620b91b456'}
commands=[('build',['cargo','build','--release','-p','rustuml','--bin','rustuml','-p','rustuml-oracle','--example','diff_one']),('units',['cargo','test','--release','--lib','-p','rustuml-parser','-p','rustuml-render']),('cli',['cargo','test','--release','-p','rustuml','--test','pipe_mode']),('focused',['cargo','test','--release','-p','rustuml-render','--lib','sequence::tests']),('main-tiers-and-guards',['cargo','test','--release','--no-fail-fast','-p','rustuml-oracle','--test','golden_no_oracle','--test','golden_pairs','--test','parity_perturbations','--test','harness_guards','--','--nocapture']),('format-owned',['rustfmt','--check','--edition','2024','--config','skip_children=true','crates/rustuml-render/src/sequence.rs']),('format-workspace',['cargo','fmt','--all','--','--check']),('clippy',['cargo','clippy','-p','rustuml-parser','-p','rustuml-render','--','-D','warnings']),('clippy-render',['cargo','clippy','-p','rustuml-render','--no-deps','--','-D','warnings'])]
records=[]
try:
 for label,rev in revisions.items():
  subprocess.run(['git','switch','--detach',rev],cwd=ROOT,check=True)
  logs=CACHE/(label+'-logs');logs.mkdir(exist_ok=False)
  target=CACHE/(label+'-target');bins=CACHE/(label+'-bin');bins.mkdir()
  env=dict(os.environ,CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',CARGO_TARGET_DIR=str(target),PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
  for name,argv in commands:
   start=time.time()
   with (logs/(name+'.log')).open('wb') as f:r=subprocess.run(argv,cwd=ROOT,env=env,stdout=f,stderr=subprocess.STDOUT)
   records.append(dict(revision=rev,label=label,name=name,command=argv,cwd=str(ROOT),environment={k:env[k] for k in ['CARGO_BUILD_JOBS','RAYON_NUM_THREADS','CARGO_TARGET_DIR','PLANTUML_STDLIB']},exit=r.returncode,seconds=time.time()-start))
   (CACHE/'checks.json').write_text(json.dumps(records,indent=2)+'\n');print(label,name,r.returncode,flush=True)
   if name=='main-tiers-and-guards':
    for filename in ['no_oracle_failure_names.txt','golden_failure_names.txt']:
     p=ROOT/'test-diagrams'/filename
     if p.exists():shutil.copy2(p,logs/filename)
   if name=='build':
    assert r.returncode==0
    for src,dst in [(target/'release/rustuml','rustuml'),(target/'release/examples/diff_one','diff_one')]:shutil.copy2(src,bins/dst)
    rlib=list((target/'release/deps').glob('librustuml_oracle-*.rlib'));assert len(rlib)==1
    argv=['rustc','--edition=2024',str(CACHE/'compare_cli.rs'),'--extern','rustuml_oracle='+str(rlib[0]),'-L','dependency='+str(target/'release/deps'),'-o',str(bins/'compare_cli')]
    subprocess.run(argv,cwd=ROOT,env=env,check=True)
    (bins/'binding.json').write_text(json.dumps(dict(revision=rev,comparator_command=argv,sha256={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in bins.iterdir()}),indent=2)+'\n')
finally:subprocess.run(['git','switch',BRANCH],cwd=ROOT,check=True)
