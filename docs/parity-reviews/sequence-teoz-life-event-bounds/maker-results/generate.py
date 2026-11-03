import concurrent.futures,json,subprocess
from pathlib import Path
out=Path('/tmp/rustuml-seq-recovery/fresh-teoz-life');out.mkdir(exist_ok=True)
java='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java';jar='/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
rows=[]
for reverse in (False,True):
 for depth in (1,3):
  for form in ('explicit','message','preexisting'):
   for close_inside in (False,True):
    body='!pragma teoz true\n'+('participant Cedar as C\nparticipant Pine as P\n' if reverse else 'participant Pine as P\nparticipant Cedar as C\n')
    if form=='preexisting': body+='activate C\n'*depth
    body+='group frame\nP -> C : request\n'
    if form=='explicit': body+='activate C\n'*depth
    if form=='message': body+='C -> C ++ : enter\n'*depth
    body+='note right of P : fractional iWi\n'
    if close_inside: body+='deactivate C\n'*depth
    body+='C -> P : completion\nend\n'
    if not close_inside: body+='deactivate C\n'*depth
    rows.append((f'life_{len(rows)}',body))
def run(row):
 name,body=row;p=out/(name+'.puml');src='@startuml\n'+body+'@enduml\n';p.write_text(src)
 j=subprocess.run([java,'-Djava.awt.headless=true','-jar',jar,'-tsvg','-pipe','-charset','UTF-8'],input=src,text=True,capture_output=True,timeout=60);p.with_suffix('.svg').write_text(j.stdout)
 return {'source':p.name,'java_exit':j.returncode,'java_stderr':j.stderr}
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool: results=list(pool.map(run,rows))
(out/'generated.json').write_text(json.dumps(results,indent=2)+'\n');print(len(results),'cases',sum(r['java_exit']!=0 for r in results),'Java errors')
