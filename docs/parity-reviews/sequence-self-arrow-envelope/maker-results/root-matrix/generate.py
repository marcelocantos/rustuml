import concurrent.futures,json,subprocess
from pathlib import Path
out=Path('/tmp/rustuml-seq-recovery/fresh-root-envelope');out.mkdir(exist_ok=True)
java='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java';jar='/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
rows=[]')[0].replace("fresh-envelope","fresh-root-envelope"))
rows=[]
for depth in (0,1,2,4):
 for label in ('q','WWW thick label WWW'):
  for mode in ('single','after','before','created','destroyed','attached'):
   body='participant Lead as L\n' if mode in ('after','created','destroyed','attached') else ''
   if mode=='created':body+='create S\nL -> S : birth\n'
   else:body+='participant S\n'
   if mode=='before':body+='participant Tail as T\n'
   body+=''.join('S -> S ++ : recursive\n' for _ in range(depth))+'S -> S : '+label+'\n'
   if mode=='attached':body+='note right : Held WWW\n'
   body+=''.join('return unwind\n' for _ in range(depth))
   if mode=='destroyed':body+='L -> S !! : destroy\n'
   rows.append((f'root_{len(rows)}',body))
def run(row):
 name,body=row;p=out/(name+'.puml');src='@startuml\n'+body+'@enduml\n';p.write_text(src)
 j=subprocess.run([java,'-Djava.awt.headless=true','-jar',jar,'-tsvg','-pipe','-charset','UTF-8'],input=src,text=True,capture_output=True,timeout=60);p.with_suffix('.svg').write_text(j.stdout)
 return {'source':p.name,'java_exit':j.returncode,'java_stderr':j.stderr}
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool: results=list(pool.map(run,rows))
(out/'generated.json').write_text(json.dumps(results,indent=2)+'\n');print(len(results),'cases',sum(r['java_exit']!=0 for r in results),'Java errors')
