# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib, json, os, shutil, subprocess
from pathlib import Path

ROOT = Path('/Users/marcelo/.t3/worktrees/rustuml/t3-607d50f7')
CACHE = Path(__file__).parent
REVS = [('before','efc2e922bbd093cf13d827501f5e53aa850a9e68'), ('intermediate','350e776a2cd376dd2171dc79b61010da8f01fc2e'), ('after','092cf5f68464625c8b3771e069b42f44f43fc0c9')]
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def git(*args): return subprocess.check_output(['git',*args],cwd=ROOT,text=True).strip()
for name, rev in REVS:
    assert not git('status','--porcelain')
    subprocess.run(['git','switch','--detach',rev],cwd=ROOT,check=True)
    target = CACHE / ('target-'+name)
    dest = CACHE / ('bin-'+name)
    dest.mkdir(exist_ok=True)
    env = dict(os.environ, CARGO_BUILD_JOBS='4', RAYON_NUM_THREADS='4', CARGO_TARGET_DIR=str(target), PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib', RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
    argv = ['cargo','build','--release','-p','rustuml','--bin','rustuml','-p','rustuml-oracle','--example','diff_one']
    with (CACHE/(name+'-build.log')).open('wb') as log:
        p = subprocess.run(argv,cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
    record = dict(revision=git('rev-parse','HEAD'), tree=git('rev-parse','HEAD^{tree}'), status=git('status','--porcelain'), command=argv, exit=p.returncode, cwd=str(ROOT), environment={k:env[k] for k in ['CARGO_BUILD_JOBS','RAYON_NUM_THREADS','CARGO_TARGET_DIR','PLANTUML_STDLIB','RUSTUML_DEBUG']})
    assert p.returncode==0, record
    for src, dst in [(target/'release/rustuml', dest/'rustuml'), (target/'release/examples/diff_one',dest/'diff_one')]: shutil.copy2(src,dst)
    libs=list((target/'release/deps').glob('librustuml_oracle-*.rlib')); assert len(libs)==1
    adapter=ROOT/'docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs'
    command=['rustc','--edition=2024',str(adapter),'--extern','rustuml_oracle='+str(libs[0]),'-L','dependency='+str(target/'release/deps'),'-o',str(dest/'compare_cli')]
    subprocess.run(command,cwd=ROOT,env=env,check=True)
    record['comparator_build']=command
    record['binaries']={p.name:sha(p) for p in dest.iterdir()}
    record['source_files']={p:sha(ROOT/p) for p in git('ls-files','crates','Cargo.lock','Cargo.toml').splitlines() if (ROOT/p).is_file()}
    (CACHE/(name+'-build.json')).write_text(json.dumps(record,indent=2)+'\n')
    print(name,'built',record['revision'],flush=True)
subprocess.run(['git','switch','review/sequence-092cf5f-astra-20261104'],cwd=ROOT,check=True)
