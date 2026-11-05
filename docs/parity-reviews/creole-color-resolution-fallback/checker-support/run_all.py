import pathlib,subprocess,json
S=pathlib.Path(__file__).parent
cohorts=['retained','maker18','sprites50','checker38','fresh15','fresh26','new15','delimiters4','focused2','checker30','supplement8','fresh12','checker28','fresh16','golden-regressions','checker36','fallback16']
for phase in ['before','after']:
 q=subprocess.run(['uv','run','--no-project','--with','pyyaml','python',str(S/'run_replay.py'),phase,*cohorts],cwd=S)
 assert q.returncode==0
