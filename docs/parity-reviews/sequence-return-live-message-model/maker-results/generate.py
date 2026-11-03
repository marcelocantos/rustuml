import concurrent.futures, json, subprocess
from pathlib import Path
out=Path('/tmp/rustuml-seq-recovery/fresh-returns');out.mkdir(exist_ok=True)
java='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java'
jar='/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
rows=[]
for order in ('CSD','DSC','SDC'):
 for depth in (1,3):
  for arrow,label in [('->','done'),('->>','completed broad message WWWiii'),('-[#purple]>','result of delegation')]:
   body=''.join('participant '+p+'\n' for p in order)+'C '+arrow+' S ++ : entry\n'+('S '+arrow+' S ++ : recursion\n')*depth+'S '+arrow+' D ++ : dispatch\nreturn '+label+'\n'+('return local result\n')*depth+'return complete\n'
   rows.append((f'return_{len(rows)}',body,['participant order','activation depth','arrow configuration and label width']))
for order in ('CSD','DSC'):
 body=''.join('participant '+p+'\n' for p in order)+'C ->> S : question\nreturn answer\nreturn reconsider\n'
 rows.append((f'fallback_{len(rows)}',body,['participant order','fallback without activation','alternating return ownership']))
def run(row):
 name,body,axes=row;source='@startuml\n'+body+'@enduml\n';p=out/(name+'.puml');p.write_text(source)
 result=subprocess.run([java,'-Djava.awt.headless=true','-jar',jar,'-tsvg','-pipe','-charset','UTF-8'],input=source,text=True,capture_output=True,timeout=60)
 p.with_suffix('.svg').write_text(result.stdout)
 return {'source':p.name,'axes':axes,'java_exit':result.returncode,'java_stderr':result.stderr}
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool: results=list(pool.map(run,rows))
(out/'generated.json').write_text(json.dumps(results,indent=2)+'\n')
print('generated',len(results),'valid exits',sum(r['java_exit']==0 for r in results))
