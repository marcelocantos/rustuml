import concurrent.futures,json,subprocess
from pathlib import Path
out=Path('/tmp/rustuml-seq-recovery/fresh-async');out.mkdir(exist_ok=True)
java='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java';jar='/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
rows=[]
for head in ('>','>>','//'):
 for depth in (0,1,3):
  for wide in (False,True):
   body='participant Caller\nparticipant Work\nparticipant Receiver\n'
   if depth: body+='Caller -> Work ++ : enter\n'+('Work -> Work ++ : nest\n')*(depth-1)
   body+='Work -'+head+' Work : '+('WWWiii extended self message' if wide else 'short')+'\n'
   body+='Work -'+head+' Work ++ : activate\nreturn close local\n'
   if depth: body+='return unwind\n'*depth
   rows.append((f'async_{len(rows)}',body,['head style','live depth','label width','call and return']))
def run(row):
 name,body,axes=row;src='@startuml\n'+body+'@enduml\n';p=out/(name+'.puml');p.write_text(src)
 j=subprocess.run([java,'-Djava.awt.headless=true','-jar',jar,'-tsvg','-pipe','-charset','UTF-8'],input=src,text=True,capture_output=True,timeout=60);p.with_suffix('.svg').write_text(j.stdout)
 return {'source':p.name,'axes':axes,'java_exit':j.returncode,'java_stderr':j.stderr}
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool: rows=list(pool.map(run,rows))
(out/'generated.json').write_text(json.dumps(rows,indent=2)+'\n');print(len(rows),'cases',sum(r['java_exit']!=0 for r in rows),'Java errors')
