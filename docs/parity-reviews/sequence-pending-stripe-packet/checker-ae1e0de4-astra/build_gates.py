# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import os,subprocess,json,hashlib,shutil,time
from pathlib import Path
R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-78c595f4'); C=Path(__file__).parent
records=[]
def run(name,argv,env,rev):
 out=C/rev;out.mkdir(exist_ok=True)
 start=time.time()
 with (out/(name+'.log')).open('wb') as f:p=subprocess.run(argv,cwd=R,env=env,stdout=f,stderr=subprocess.STDOUT)
 records.append(dict(revision=rev,name=name,argv=argv,cwd=str(R),exit=p.returncode,seconds=time.time()-start))
 (C/'commands.json').write_text(json.dumps(records,indent=2)); print(rev,name,p.returncode,flush=True)
 return p.returncode
for rev,commit in [('before','711dbca7c5e29592395f0927881aabfc2517b217'),('after','review/sequence-pending-stripe-independent-20261105')]:
 subprocess.run(['git','switch','--detach',commit] if rev=='before' else ['git','switch',commit],cwd=R,check=True)
 env=dict(os.environ,CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',CARGO_TARGET_DIR=str(C/('target-'+rev)),PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
 assert run('build',['cargo','build','--release','-p','rustuml','--bin','rustuml','-p','rustuml-oracle','--example','diff_one'],env,rev)==0
 b=C/(rev+'-bin');b.mkdir()
 for src,dst in [('rustuml','rustuml'),('examples/diff_one','diff_one')]:shutil.copy2(Path(env['CARGO_TARGET_DIR'])/'release'/src,b/dst)
 deps=Path(env['CARGO_TARGET_DIR'])/'release/deps';libs=list(deps.glob('librustuml_oracle-*.rlib'));assert len(libs)==1
 assert run('comparator',['rustc','--edition','2024',str(C/'compare_cli.rs'),'--extern','rustuml_oracle='+str(libs[0]),'-L','dependency='+str(deps),'-o',str(b/'compare_cli')],env,rev)==0
 (C/rev/'binding.json').write_text(json.dumps(dict(head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=R,text=True).strip(),status=subprocess.check_output(['git','status','--short'],cwd=R,text=True),env={k:env[k] for k in ['CARGO_BUILD_JOBS','RAYON_NUM_THREADS','CARGO_TARGET_DIR','PLANTUML_STDLIB']},binaries={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in b.iterdir()}),indent=2))
 for name,argv in [('units',['cargo','test','--release','--lib','-p','rustuml-parser','-p','rustuml-render']),('cli',['cargo','test','--release','-p','rustuml','--test','pipe_mode']),('tiers',['cargo','test','--release','--no-fail-fast','-p','rustuml-oracle','--test','golden_no_oracle','--test','golden_pairs','--test','parity_perturbations','--test','harness_guards','--','--nocapture']),('fmt',['cargo','fmt','--all','--','--check']),('clippy',['cargo','clippy','-p','rustuml-parser','-p','rustuml-render','--','-D','warnings']),('clippy-render',['cargo','clippy','-p','rustuml-render','--no-deps','--','-D','warnings'])]:
  run(name,argv,env,rev)
  if name=='tiers':
   for f in ['no_oracle_failure_names.txt','golden_failure_names.txt']:
    if (R/'test-diagrams'/f).exists():shutil.copy2(R/'test-diagrams'/f,C/rev/f)
print('BUILD_GATES_COMPLETE',flush=True)
