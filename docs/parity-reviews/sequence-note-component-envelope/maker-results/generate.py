import concurrent.futures,json,subprocess
from pathlib import Path
out=Path('/tmp/rustuml-seq-recovery/fresh-shadow');out.mkdir(exist_ok=True)
java='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java';jar='/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
rows=[]
for style in ('skinparam shadowing false','skinparam shadowing true','skinparam NoteShadowing true'):
 for pad in (0,4):
  for position in ('left of R','right of R','over R','left','right'):
   for reverse in (False,True):
    declarations='participant Branch as R\nparticipant Root as L\n' if reverse else 'participant Root as L\nparticipant Branch as R\n'
    body=style+f'\nskinparam Padding {pad}\n'+declarations+'group Outer\nL -> R ++ : borrow\n'+f'note {position}\nWi matrix\nsecond line\nend note\n'+'opt Held\nR -> R ++ : nested\nreturn inside\nreturn outside\nend\nend\n'
    rows.append((f'shadow_{len(rows)}',body))
def run(row):
 name,body=row;p=out/(name+'.puml');src='@startuml\n'+body+'@enduml\n';p.write_text(src)
 j=subprocess.run([java,'-Djava.awt.headless=true','-jar',jar,'-tsvg','-pipe','-charset','UTF-8'],input=src,text=True,capture_output=True,timeout=60);p.with_suffix('.svg').write_text(j.stdout)
 return {'source':p.name,'java_exit':j.returncode,'java_stderr':j.stderr}
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool: results=list(pool.map(run,rows))
(out/'generated.json').write_text(json.dumps(results,indent=2)+'\n');print(len(results),'cases',sum(r['java_exit']!=0 for r in results),'Java errors')
