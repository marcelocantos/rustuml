import pathlib,subprocess,os,json,time,shutil,hashlib
S=pathlib.Path(__file__).parent; R=pathlib.Path('/Users/marcelo/.t3/worktrees/rustuml/t3-ae47bffd');records=[]
for phase in ['before','after']:
 d=S/('src-'+phase);g=d/'test-diagrams/golden';g.rmdir();g.symlink_to(R/'test-diagrams/golden',target_is_directory=True)
 env=dict(os.environ,CARGO_TARGET_DIR=str(S/('target-'+phase)),CARGO_BUILD_JOBS='4')
 for name,argv in [('snapshot-make',['make','build']),('build',['cargo','build','--release','-p','rustuml','-j','4'])]:
  if name=='snapshot-make':
   # Inspect the existing Makefile; build only Cargo with vendored generated headers.
   continue
  start=time.monotonic()
  with (S/'logs'/f'{name}-{phase}.log').open('wb') as f:q=subprocess.run(argv,cwd=d,env=env,stdout=f,stderr=subprocess.STDOUT)
  records.append(dict(phase=phase,argv=argv,cwd=str(d),env_overrides={k:env[k] for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS']},exit=q.returncode,seconds=time.monotonic()-start))
  (S/'builds.json').write_text(json.dumps(records,indent=2))
  print(phase,name,q.returncode,flush=True)
  if q.returncode:raise SystemExit(q.returncode)
 shutil.copy2(S/('target-'+phase)/'release/rustuml',S/(phase+'-rustuml'))
