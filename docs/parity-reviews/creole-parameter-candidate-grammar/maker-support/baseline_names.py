# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,subprocess,os,shutil
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();D=S/'baseline-diagnostics';D.mkdir(exist_ok=True);rows=[];sha=lambda b:hashlib.sha256(b).hexdigest()
env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RAYON_NUM_THREADS='4',RUST_TEST_THREADS='4')
for name,exe in [('product','golden_no_oracle-901d5e92a3796558'),('assisted','golden_pairs-88a993b69eca84e7')]:
 p=S/'target-before/release/deps'/exe;original=S.parent/'rustuml-creole-hide-flush-maker/target-after/release/deps'/exe;assert p.read_bytes()==original.read_bytes()
 argv=[str(p),'--nocapture'];q=subprocess.run(argv,env=env,capture_output=True);(D/(name+'.stdout')).write_bytes(q.stdout);(D/(name+'.stderr')).write_bytes(q.stderr);rows.append(dict(name=name,argv=argv,cwd=str(R),env_overrides={k:env[k] for k in ['RUSTUML_DEBUG','PLANTUML_STDLIB','RAYON_NUM_THREADS','RUST_TEST_THREADS']},binary_sha256=sha(p.read_bytes()),prior_maker_binary=str(original),matches_prior_binary=True,prior_production='a6e1718c4eb693aec99c3a107fd654afb8a4e4e3',exit=q.returncode))
 filename='no_oracle_failure_names.txt' if name=='product' else 'golden_failure_names.txt';shutil.copy2(R/'test-diagrams'/filename,D/filename)
 before=set((D/filename).read_text().splitlines());initial=set((S/'initial-broad'/filename).read_text().splitlines());rows[-1].update(initial_losses=sorted(initial-before),initial_gains=sorted(before-initial))
(S/'baseline-diagnostics.json').write_text(json.dumps(rows,indent=2)+'\n');print(json.dumps(rows,indent=2))
