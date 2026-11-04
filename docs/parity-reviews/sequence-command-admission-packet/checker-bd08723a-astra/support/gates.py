# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,os,subprocess,shutil
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();P=R/'docs/parity-reviews/sequence-command-admission-packet/maker-results';D=C/'gates';D.mkdir(exist_ok=False)
env=dict(os.environ,CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',CARGO_TARGET_DIR=str(C/'target-after'),PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',JAVA_TOOL_OPTIONS=(os.environ.get('JAVA_TOOL_OPTIONS','')+' -Djava.awt.headless=true').strip())
records=[]
for item in json.loads((P/'final-checks.json').read_text()):
    name=item['name'];cmd=item['command']
    with (D/(name+'.log')).open('wb') as f:p=subprocess.run(cmd,cwd=R,env=env,stdout=f,stderr=subprocess.STDOUT)
    records.append(dict(name=name,command=cmd,cwd=str(R),exit=p.returncode,revision=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()))
    (D/'commands.json').write_text(json.dumps(records,indent=2)+'\n')
    if name=='main-tiers-and-guards':
        for filename in ['no_oracle_failure_names.txt','golden_failure_names.txt']:
            shutil.copy2(R/'test-diagrams'/filename,D/filename)
    print(name,p.returncode,flush=True)
