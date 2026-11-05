import concurrent.futures,json,subprocess
from pathlib import Path
out=Path('/tmp/rustuml-seq-recovery/fresh-page');out.mkdir(exist_ok=True)
java='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java';jar='/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
rows=[]
for depth in (1,2,4):
 for placement in ('before','after','across'):
  body='participant Willow as W\nparticipant Fir as F\nW -> F : open\n'
  if placement=='after':body+='newpage next\n'
  body+=''.join('F -> F ++ : acquire\n' for _ in range(depth))
  if placement=='across':body+='newpage next\n'
  body+=''.join('return finish\n' for _ in range(depth))
  if placement=='before':body+='newpage next\n'
  body+='F -> W : continued WWWWWWWW\n';rows.append((f'page_{len(rows)}',body))
for mode in ('plain','group','notes'):
 for title in ('Small','Wide heading WWWWWWWWWWWWWWWWWWWWWWWWW','First line\\nLong second heading WWWWWWWWWWWWWWWW'):
  for chrome in ('','caption Caption WWWWWWWWWWWWWW\nheader Heading WWWWWWWWWWWWWWWWW\nfooter Footer ffff\n'):
   body='title '+title+'\n'+chrome+'participant Willow as W\nparticipant Fir as F\n'
   if mode!='plain':body+='group package\n'
   if mode=='notes':body+='note left of W : Wide outside note WWW\n'
   body+='W -> F : request\nF --> W : response\n'
   if mode!='plain':body+='end\n'
   rows.append((f'page_{len(rows)}',body))
def run(row):
 name,body=row;p=out/(name+'.puml');src='@startuml\n'+body+'@enduml\n';p.write_text(src)
 j=subprocess.run([java,'-Djava.awt.headless=true','-jar',jar,'-tsvg','-pipe','-charset','UTF-8'],input=src,text=True,capture_output=True,timeout=60);p.with_suffix('.svg').write_text(j.stdout)
 return {'source':p.name,'java_exit':j.returncode,'java_stderr':j.stderr}
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool: results=list(pool.map(run,rows))
(out/'generated.json').write_text(json.dumps(results,indent=2)+'\n');print(len(results),'cases',sum(r['java_exit']!=0 for r in results),'Java errors')
