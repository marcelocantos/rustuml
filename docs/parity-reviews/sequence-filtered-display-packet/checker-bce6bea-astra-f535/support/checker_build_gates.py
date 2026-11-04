# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,os,shutil,subprocess,time
from pathlib import Path
R=Path.cwd(); C=Path(__file__).parent; O=C/'independent';O.mkdir(exist_ok=True)
helper=R/'docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs'
shutil.copy2(helper,O/'compare_cli.rs')
records=[]
def run(name,args,env):
 start=time.time()
 with (O/(name+'.log')).open('wb') as f:p=subprocess.run(args,cwd=R,env=env,stdout=f,stderr=subprocess.STDOUT)
 records.append(dict(name=name,argv=args,cwd=str(R),revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=R,text=True).strip(),exit=p.returncode,seconds=time.time()-start,env={k:env[k] for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS','RAYON_NUM_THREADS','RUSTUML_DEBUG','PLANTUML_STDLIB'] if k in env}))
 (O/'commands.json').write_text(json.dumps(records,indent=2)+'\n');print(name,p.returncode,flush=True)
 return p.returncode
for label,rev in [('before','0cde5d7e55139b63cb8e91f8e42acd473441f80a'),('after','bce6bea405d8b5efd55136f4768117115e7f634f')]:
 subprocess.run(['git','switch','--detach',rev],check=True,cwd=R)
 target=C/('target-'+label);env=dict(os.environ,CARGO_TARGET_DIR=str(target),CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
 assert run(label+'-build',['cargo','build','--release','-j','4','-p','rustuml','-p','rustuml-oracle','--example','diff_one','--bin','rustuml'],env)==0
 bins=O/(label+'-bin');bins.mkdir(exist_ok=True)
 shutil.copy2(target/'release/rustuml',bins/'rustuml');shutil.copy2(target/'release/examples/diff_one',bins/'diff_one')
 rlibs=list((target/'release/deps').glob('librustuml_oracle-*.rlib'));assert len(rlibs)==1
 assert run(label+'-comparator',['rustc','--edition=2024',str(O/'compare_cli.rs'),'--extern','rustuml_oracle='+str(rlibs[0]),'-L','dependency='+str(target/'release/deps'),'-o',str(bins/'compare_cli')],env)==0
 (O/(label+'-binding.json')).write_text(json.dumps(dict(revision=rev,binaries={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in bins.iterdir()}),indent=2)+'\n')
 commands=[('units',['cargo','test','--release','--lib','-p','rustuml-parser','-p','rustuml-render']),('cli',['cargo','test','--release','-p','rustuml','--test','pipe_mode']),('main-tiers-and-guards',['cargo','test','--release','--no-fail-fast','-p','rustuml-oracle','--test','golden_no_oracle','--test','golden_pairs','--test','parity_perturbations','--test','harness_guards','--','--nocapture']),('format-owned',['rustfmt','--check','--edition','2024','--config','skip_children=true','crates/rustuml-parser/src/diagram/sequence.rs','crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs','crates/rustuml-render/src/ascii.rs']),('format-workspace',['cargo','fmt','--all','--','--check']),('clippy',['cargo','clippy','-p','rustuml-parser','-p','rustuml-render','--','-D','warnings']),('clippy-render',['cargo','clippy','-p','rustuml-render','--no-deps','--','-D','warnings'])]
 for name,argv in commands:
  run(label+'-'+name,argv,env)
  if name=='main-tiers-and-guards':
   dest=O/(label+'-failure-names');dest.mkdir(exist_ok=True)
   for filename in ['no_oracle_failure_names.txt','golden_failure_names.txt']:shutil.copy2(R/'test-diagrams'/filename,dest/filename)
subprocess.run(['git','switch','review/sequence-packet-independent-f535ce04'],check=True,cwd=R)
