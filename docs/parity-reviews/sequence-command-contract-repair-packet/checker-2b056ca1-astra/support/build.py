# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,os,shutil,subprocess,sys,time
from pathlib import Path
C=Path(__file__).parent; R=Path.cwd(); label=sys.argv[1]; T=C/(label+'-target'); D=C/(label+'-bin'); D.mkdir(exist_ok=False)
env=dict(os.environ,CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',CARGO_TARGET_DIR=str(T),PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
for k in ['HARNESS_WRITE_BASELINES','RUSTUML_ORACLE']:env.pop(k,None)
argv=['cargo','build','--release','--jobs','4','-p','rustuml','-p','rustuml-oracle','--bin','rustuml','--example','diff_one']
with (C/(label+'-build.log')).open('wb') as f:p=subprocess.run(argv,env=env,stdout=f,stderr=subprocess.STDOUT)
record=dict(argv=argv,exit=p.returncode,revision=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),cwd=str(R),env={k:env[k] for k in ['CARGO_BUILD_JOBS','RAYON_NUM_THREADS','CARGO_TARGET_DIR','PLANTUML_STDLIB']})
(C/(label+'-build.json')).write_text(json.dumps(record,indent=2)+'\n')
if p.returncode:sys.exit(p.returncode)
for a,b in [('rustuml','rustuml'),('examples/diff_one','diff_one')]:shutil.copy2(T/'release'/a,D/b)
if label=='before':
 lib=list((T/'release/deps').glob('librustuml_oracle-*.rlib'));assert len(lib)==1
 argv=['rustc','--edition=2024','docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs','-L',str(T/'release/deps'),'--extern','rustuml_oracle='+str(lib[0]),'-o',str(D/'compare_cli')]
 subprocess.run(argv,check=True);record['comparator_build']=argv
else:shutil.copy2(C/'before-bin/compare_cli',D/'compare_cli')
sha=lambda b:hashlib.sha256(b).hexdigest()
record.update(binaries={p.name:sha(p.read_bytes()) for p in D.iterdir()},source_files={p:sha((R/p).read_bytes()) for p in subprocess.check_output(['git','ls-files','crates'],text=True).splitlines() if (R/p).is_file()})
(C/(label+'-build.json')).write_text(json.dumps(record,indent=2)+'\n');print(label,'built',record['revision'],flush=True)
