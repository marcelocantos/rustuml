import concurrent.futures,json,subprocess
from pathlib import Path
out=Path('/tmp/rustuml-seq-recovery/fresh-external');out.mkdir(exist_ok=True)
java='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java';jar='/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
rows=[]
def line(border,first,incoming,opened,dotted=False):
 rightward=incoming if first else not incoming
 arrow=('--' if dotted else '-')+('>>' if opened else '>') if rightward else ('<<' if opened else '<')+('--' if dotted else '-')
 return border+arrow+' Probe' if first else 'Probe '+arrow+border
for border in ('[',']'):
 for first in (False,True):
  for incoming in (False,True):
   for opened in (False,True):
    for life in ('none','explicit','auto'):
     body='participant Probe\nparticipant Relay\n'+('autoactivate on\n' if life=='auto' else '')+'group gate\n'
     body+=line(border,first,incoming,opened)+(' ++' if life=='explicit' else '')+' : crossing Wi.f\n'
     body+=line(border,not first,not incoming,opened,True)+(' --' if life=='explicit' else '')+' : settle\nend\n'
     rows.append((f'border_{len(rows)}',body,['physical border','textual direction','head style','activation mode']))
for border in ('[',']'):
 for incoming in (False,True):
  for opened in (False,True):
   body='participant Probe\nparticipant Relay\n'+line(border,True,incoming,opened)+' ++ : enter\nreturn boundary result\n'
   rows.append((f'return_{len(rows)}',body,['border','return ownership','head style']))
def run(row):
 name,body,axes=row;src='@startuml\n'+body+'@enduml\n';p=out/(name+'.puml');p.write_text(src)
 j=subprocess.run([java,'-Djava.awt.headless=true','-jar',jar,'-tsvg','-pipe','-charset','UTF-8'],input=src,text=True,capture_output=True,timeout=60)
 p.with_suffix('.svg').write_text(j.stdout)
 return {'source':p.name,'axes':axes,'java_exit':j.returncode,'java_stderr':j.stderr}
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool: rows=list(pool.map(run,rows))
(out/'generated.json').write_text(json.dumps(rows,indent=2)+'\n');print(len(rows),'cases',sum(r['java_exit']!=0 for r in rows),'Java errors')
