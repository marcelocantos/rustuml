import concurrent.futures, json, subprocess
from pathlib import Path
out=Path('/tmp/rustuml-seq-recovery/fresh-self');out.mkdir(exist_ok=True)
java='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java'
jar='/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
rows=[]
for order in [('S','R','T'),('R','S','T'),('R','T','S')]:
 for depth in (1,2,4):
  for long in (False,True):
   decl=''.join('participant '+p+'\n' for p in order)
   label='restore WMWM buffer several times' if long else 'restore'
   body='R -> S ++ : enter\n'+'S -> S ++ : recurse\n'*depth+'return '+label+'\n'
   body+='return resume\n'*(depth-1)+'return exit\n'
   rows.append((f'order_{"".join(order)}_depth_{depth}_long_{int(long)}',decl+body,['participant order','self recursion depth','return label width']))
for arrow in ('->','->>','-[#purple]>'):
 for activate in (False,True):
  marker=' ++' if activate else ''
  rows.append((f'single_{len(rows)}','S '+arrow+' S'+marker+' : start\nreturn settled\n',['one participant','activation versus fallback','inherited arrow style']))
for order in [('S','R','T'),('R','T','S')]:
 body=''.join('participant '+p+'\n' for p in order)+'R -> S ++ : enter\nS -> S ++ : recursive\nS -> T ++ : delegated\nreturn result\nreturn local\nreturn caller\n'
 rows.append((f'delegated_{len(rows)}',body,['participant order','return ownership','nested delegation']))
body='participant S\nparticipant R\nS -> R : first page\nnewpage Continuation\nS -> S ++ : call\nreturn second page unwind with wide label WWWWWW\n'
rows.append(('later_page',body,['page boundary','self return width']))
def run(row):
 name,body,axes=row;source='@startuml\n'+body+'@enduml\n';p=out/(name+'.puml');p.write_text(source)
 j=subprocess.run([java,'-Djava.awt.headless=true','-jar',jar,'-tsvg','-pipe','-charset','UTF-8'],input=source,text=True,capture_output=True,timeout=60)
 p.with_suffix('.svg').write_text(j.stdout)
 return {'source':p.name,'axes':axes,'java_exit':j.returncode,'java_stderr':j.stderr}
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool: result=list(pool.map(run,rows))
(out/'generated.json').write_text(json.dumps(result,indent=2)+'\n')
print('generated',len(result),'valid exits',sum(r['java_exit']==0 for r in result))
