from pathlib import Path
import subprocess,os,json,sys,concurrent.futures
phase=sys.argv[1];binary='/tmp/rustuml-cff2-before-rustuml' if phase=='before' else '/tmp/rustuml-cff-target/release/rustuml'
env=os.environ.copy();env.update(PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
def run(group):
 args=['python3','docs/parity-reviews/class-native-container-factory-precedence/checker-evidence/check_cli.py',binary,'/tmp/rustuml-cff-check-svg','/tmp/rustuml-cff2/'+group,'/tmp/rustuml-cff2/'+phase+'/'+group]
 r=subprocess.run(args,env=env,capture_output=True,text=True);Path('/tmp/rustuml-cff2/'+phase+'-'+group+'.log').write_text(r.stdout+r.stderr)
 results=json.loads(Path(args[-1]+'/results.json').read_text());return {'group':group,'argv':args,'exit':r.returncode,'pass':sum(x['cli_exit']==x['ast_exit']==x['comparison_exit']==0 for x in results.values()),'parse_errors':[n for n,x in results.items() if x['ast_exit']],'total':len(results)}
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:results=list(pool.map(run,['loss36','checker23','prior31','theme33']))
Path('/tmp/rustuml-cff2/'+phase+'-commands.json').write_text(json.dumps(results,indent=2)+'\n');print(json.dumps(results,indent=2))
