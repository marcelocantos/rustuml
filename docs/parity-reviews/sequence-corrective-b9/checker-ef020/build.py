# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import subprocess, pathlib, os, json, hashlib, shutil
root=pathlib.Path.cwd(); c=pathlib.Path('/Users/marcelo/Library/Caches/rustuml-sequence-check-ef020aa1'); records=[]
def run(argv,env,name):
 with (c/(name+'.log')).open('wb') as f: p=subprocess.run(argv,cwd=root,env=env,stdout=f,stderr=subprocess.STDOUT)
 records.append(dict(argv=argv,cwd=str(root),exit=p.returncode,log=name+'.log')); (c/'build-commands.json').write_text(json.dumps(records,indent=2)); print(name,p.returncode,flush=True)
 if p.returncode: raise RuntimeError(name)
try:
 for label,ref in [('before','5358314a0180cf760584c79bb5efd4ec1ae6320b'),('after','review/sequence-corrective-efc2e922-independent')]:
  run(['git','switch','--detach',ref] if label=='before' else ['git','switch',ref],os.environ,label+'-switch')
  target=c/(label+'-target'); env=dict(os.environ,CARGO_TARGET_DIR=str(target),CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4')
  for key in ['HARNESS_WRITE_BASELINES','RUSTUML_ORACLE','RUSTC_WRAPPER']: env.pop(key,None)
  run(['cargo','build','--locked','--release','-p','rustuml','-p','rustuml-oracle','--bin','rustuml','--example','diff_one'],env,label+'-build')
  bins=c/(label+'-bin'); bins.mkdir(); shutil.copy2(target/'release/rustuml',bins/'rustuml');shutil.copy2(target/'release/examples/diff_one',bins/'diff_one')
  rlib=list((target/'release/deps').glob('librustuml_oracle-*.rlib'));assert len(rlib)==1
  run(['rustc','--edition=2024',str(c/'compare_cli.rs'),'--extern','rustuml_oracle='+str(rlib[0]),'-L','dependency='+str(target/'release/deps'),'-o',str(bins/'compare_cli')],env,label+'-comparator')
  (c/(label+'-binaries.json')).write_text(json.dumps(dict(revision=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),target=str(target),files={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in bins.iterdir()}),indent=2))
finally:
 subprocess.run(['git','switch','review/sequence-corrective-efc2e922-independent'],cwd=root,check=True)
