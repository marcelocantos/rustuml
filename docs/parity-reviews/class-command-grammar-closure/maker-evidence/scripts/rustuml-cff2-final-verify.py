# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
import subprocess,os,json,concurrent.futures,hashlib,time
root=Path.cwd();out=Path('/tmp/rustuml-cff2/final-validation');out.mkdir(exist_ok=True)
env=os.environ.copy();env.update(CARGO_TARGET_DIR='/tmp/rustuml-cff-target',CARGO_BUILD_JOBS='4',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
records=[]
def run(name,args):
 start=time.time()
 with (out/(name+'.log')).open('w') as f:r=subprocess.run(args,cwd=root,env=env,stdout=f,stderr=subprocess.STDOUT)
 row={'name':name,'argv':args,'cwd':str(root),'exit':r.returncode,'elapsed_seconds':round(time.time()-start,3),'log':name+'.log'};print(row,flush=True);return row
records.append(run('build',['cargo','build','--release','-p','rustuml']))
if records[-1]['exit']:raise SystemExit(1)
cli='/tmp/rustuml-cff-target/release/rustuml'
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
 futures=[pool.submit(run,'cli833',['python3','/tmp/rustuml-cff2-inventory.py','after']),pool.submit(run,'cli-relevant',['python3','/tmp/rustuml-cff2-measure.py','after']),pool.submit(run,'cli-fresh',['python3','docs/parity-reviews/class-native-container-factory-precedence/checker-evidence/check_cli.py',cli,'/tmp/rustuml-cff-check-svg','/tmp/rustuml-cff2/fresh','/tmp/rustuml-cff2/after/fresh'])]
 records.extend(f.result() for f in futures)
records.append(run('workspace-lib',['cargo','test','--workspace','--lib']))
records.append(run('main-tiers-perturbations-guards',['cargo','test','--release','-p','rustuml-oracle','--test','golden_no_oracle','--test','golden_pairs','--test','parity_perturbations','--test','harness_guards','--no-fail-fast','--','--nocapture']))
records.append(run('clippy',['cargo','clippy','--workspace','--all-targets','--','-D','warnings']))
records.append(run('format',['rustfmt','--edition','2024','--check','--config','skip_children=true','crates/rustuml-parser/src/diagram/class.rs','crates/rustuml-parser/src/parse/class.rs','crates/rustuml-parser/src/parse/mod.rs','crates/rustuml-render/src/ascii_class.rs','crates/rustuml-render/src/class.rs','crates/rustuml-render/src/component.rs']))
records.append(run('diff-check',['git','diff','--check']))
(out/'commands.json').write_text(json.dumps({'implementation_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'environment':{k:env[k] for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS','PLANTUML_STDLIB','RUSTUML_DEBUG']},'binary_sha256':hashlib.sha256(Path(cli).read_bytes()).hexdigest(),'comparator_sha256':hashlib.sha256(Path('/tmp/rustuml-cff-check-svg').read_bytes()).hexdigest(),'results':records},indent=2)+'\n')
# This is a measurement driver. Individual failing exits above are retained,
# not converted into a successful oracle/acceptance claim.
