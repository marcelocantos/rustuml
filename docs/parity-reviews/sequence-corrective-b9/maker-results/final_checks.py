# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,os,subprocess
from pathlib import Path
root=Path.cwd();cache=Path('/Users/marcelo/Library/Caches/rustuml-sequence-corrective-b9');logs=cache/'final-logs';logs.mkdir(exist_ok=True)
env=dict(os.environ,CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',CARGO_TARGET_DIR='/Users/marcelo/Library/Caches/rustuml-sequence-repair/target',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',JAVA_TOOL_OPTIONS=(os.environ.get('JAVA_TOOL_OPTIONS','')+' -Djava.awt.headless=true').strip())
commands=[('build',['cargo','build','--release','-p','rustuml','--bin','rustuml','-p','rustuml-oracle','--example','diff_one']),('units',['cargo','test','--release','--lib','-p','rustuml-parser','-p','rustuml-render']),('cli',['cargo','test','--release','-p','rustuml','--test','pipe_mode']),('main-tiers-and-guards',['cargo','test','--release','--no-fail-fast','-p','rustuml-oracle','--test','golden_no_oracle','--test','golden_pairs','--test','parity_perturbations','--test','harness_guards','--','--nocapture']),('format-owned',['rustfmt','--check','--edition','2024','crates/rustuml-parser/src/diagram/sequence.rs','crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs','crates/rustuml-render/src/ascii.rs']),('format-workspace',['cargo','fmt','--all','--','--check']),('clippy',['cargo','clippy','-p','rustuml-parser','-p','rustuml-render','--','-D','warnings']),('clippy-render',['cargo','clippy','-p','rustuml-render','--no-deps','--','-D','warnings'])]
records=[]
for name,argv in commands:
 with (logs/(name+'.log')).open('wb') as f: r=subprocess.run(argv,cwd=root,env=env,stdout=f,stderr=subprocess.STDOUT)
 records.append(dict(name=name,command=argv,cwd=str(root),exit=r.returncode));(cache/'final-checks.json').write_text(json.dumps(records,indent=2)+'\n');print(name,r.returncode,flush=True)
 if name=='build':
  assert r.returncode==0
  for source,name in [(Path(env['CARGO_TARGET_DIR'])/'release/rustuml','rustuml'),(Path(env['CARGO_TARGET_DIR'])/'release/examples/diff_one','diff_one')]: (cache/'bin'/name).write_bytes(source.read_bytes())
  bins={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in (cache/'bin').iterdir()}
  files=['crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs']
  (cache/'final-binaries.json').write_text(json.dumps(dict(revision=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),binaries=bins,source_files={p:hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in files},environment={k:env[k] for k in ['CARGO_BUILD_JOBS','RAYON_NUM_THREADS','CARGO_TARGET_DIR','PLANTUML_STDLIB','JAVA_TOOL_OPTIONS']}),indent=2)+'\n')
