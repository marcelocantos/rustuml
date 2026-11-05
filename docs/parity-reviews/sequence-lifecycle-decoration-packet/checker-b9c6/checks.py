# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib, json, os, pathlib, shutil, subprocess, tarfile
ROOT=pathlib.Path.cwd()
CACHE=pathlib.Path('/Users/marcelo/Library/Caches/rustuml-sequence-review-b9c6cf04')
records=[]
def run(name,argv,cwd=ROOT,target='after'):
    env=dict(os.environ,CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4',CARGO_TARGET_DIR=str(CACHE/f'target-{target}'),PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
    with (CACHE/f'{name}.log').open('wb') as f:
        p=subprocess.run(argv,cwd=cwd,env=env,stdout=f,stderr=subprocess.STDOUT)
    records.append(dict(name=name,argv=argv,cwd=str(cwd),exit=p.returncode,environment={k:env[k] for k in ('CARGO_BUILD_JOBS','RAYON_NUM_THREADS','CARGO_TARGET_DIR','PLANTUML_STDLIB')}))
    (CACHE/'checks.json').write_text(json.dumps(records,indent=2)+'\n')
    print(name,p.returncode,flush=True)
    return p.returncode
assert run('check',['cargo','check','-p','rustuml-parser','-p','rustuml-render'])==0
for name,argv in [('build-after',['cargo','build','--release','-p','rustuml','--bin','rustuml','-p','rustuml-oracle','--example','diff_one']),('units',['cargo','test','--release','--lib','-p','rustuml-parser','-p','rustuml-render']),('cli',['cargo','test','--release','-p','rustuml','--test','pipe_mode']),('main-tiers',['cargo','test','--release','--no-fail-fast','-p','rustuml-oracle','--test','golden_no_oracle','--test','golden_pairs','--test','harness_guards','--','--nocapture']),('format',['rustfmt','--check','--edition','2024','crates/rustuml-parser/src/diagram/sequence.rs','crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs','crates/rustuml-render/src/ascii.rs'])]:
    run(name,argv)
# An archive is a readonly source snapshot, not a second checkout/worktree.
before=CACHE/'before-source'; before.mkdir(exist_ok=True)
with (CACHE/'before.tar').open('wb') as f:
    subprocess.run(['git','archive','0fb4e14b7227eb768d268128d42fa5f6d5a5b1af'],cwd=ROOT,stdout=f,check=True)
with tarfile.open(CACHE/'before.tar') as t:t.extractall(before,filter='data')
run('build-before',['cargo','build','--release','-p','rustuml','--bin','rustuml','-p','rustuml-oracle','--example','diff_one'],before,'before')
for revision in ('before','after'):
    target=CACHE/f'target-{revision}'/'release'; bins=CACHE/f'bin-{revision}';bins.mkdir(exist_ok=True)
    for src,name in [(target/'rustuml','rustuml'),(target/'examples/diff_one','diff_one')]:shutil.copy2(src,bins/name)
    libs=list((target/'deps').glob('librustuml_oracle-*.rlib')); assert len(libs)==1,libs
    assert run(f'comparator-{revision}',['rustc','--edition=2024',str(ROOT/'docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs'),'--extern',f'rustuml_oracle={libs[0]}','-L',f'dependency={target / "deps"}','-o',str(bins/'compare_cli')],ROOT,revision)==0
    (CACHE/f'binaries-{revision}.json').write_text(json.dumps({p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in bins.iterdir()},indent=2)+'\n')
