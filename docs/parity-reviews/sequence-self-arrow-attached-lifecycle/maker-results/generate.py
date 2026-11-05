import concurrent.futures,json,subprocess
from pathlib import Path
out=Path('/tmp/rustuml-seq-recovery/fresh-attached');out.mkdir(exist_ok=True)
java='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java';jar='/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
rows=[]
for reverse in (False,True):
 for arrow in ('->','->>'):
  for depth in (0,2):
   for matching in (False,True):
    for form in ('activate','both','deactivate'):
     body=('participant Cedar as C\nparticipant Pine as P\n' if reverse else 'participant Pine as P\nparticipant Cedar as C\n')+'participant Willow as W\n'
     body+='P -> C ++ : seed\n'*depth
     body+=f'C {arrow} C : attached life Wi.f\n'
     target='C' if matching else 'W'
     if form!='deactivate': body+=f'activate {target}\n'
     body+='note over C, P : interleaved life\n'
     if form!='activate': body+=f'deactivate {target}\n'
     body+='C -> P : complete\n'
     rows.append((f'attached_{len(rows)}',body))
def run(row):
 name,body=row;p=out/(name+'.puml');src='@startuml\n'+body+'@enduml\n';p.write_text(src)
 j=subprocess.run([java,'-Djava.awt.headless=true','-jar',jar,'-tsvg','-pipe','-charset','UTF-8'],input=src,text=True,capture_output=True,timeout=60);p.with_suffix('.svg').write_text(j.stdout)
 return {'source':p.name,'java_exit':j.returncode,'java_stderr':j.stderr}
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool: results=list(pool.map(run,rows))
(out/'generated.json').write_text(json.dumps(results,indent=2)+'\n');print(len(results),'cases',sum(r['java_exit']!=0 for r in results),'Java errors')
