import concurrent.futures, json, os, subprocess
from pathlib import Path
out=Path('/tmp/rustuml-seq-recovery/fresh-groups');out.mkdir(exist_ok=True)
java='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java'
jar='/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
rows=[]
for depth in (1,2,4):
 for outer in ('MMMMll','WWWW lllll','Envelope abcdef'):
  for reverse in (False,True):
   decl='participant Sedge as S\nparticipant Forge as F\n'
   if reverse: decl='participant Forge as F\nparticipant Sedge as S\n'
   body='group package\nnote left of S : '+outer+'\n'+'loop cycle\n'*(depth-1)+'note left of S : WMW.ff\nS -> F : transfer\n'+'end\n'*depth
   rows.append((f'competition_{depth}_{len(rows)}',decl+body,['label widths','group depth','participant order']))
for reservation in ('note left of S : Outside WWWWWWW\n','note across : Across WWWWWWWWWWWWWWWW\n','title Enclosing title WWWWWWWWWWWWWWWW\n',''):
 for boundary in ('[-> S : incoming port\n','S ->[ : outgoing port\n',''):
  decl='participant Sedge as S\nparticipant Forge as F\n'
  if reservation.startswith('title'): decl=reservation+decl; pre=''
  else: pre=reservation
  body=pre+'group package\n'+boundary+'note left of S : WWWW.fff\nloop cycle\nnote left of S : MMMMM.i\nS -> F : transfer\nend\nend\n'
  rows.append((f'reservation_{len(rows)}',decl+body,['independent reservation','boundary direction','nested note competition']))
rows.append(('box_title','box "Wide participant container WWWWWWW"\nparticipant Sedge as S\nparticipant Forge as F\nend box\ngroup package\nnote left of S : Outside gg.ll\nS -> F : transfer\nend\n',['box title','note width']))
def run(row):
 name,body,axes=row;source='@startuml\n'+body+'@enduml\n';p=out/(name+'.puml');p.write_text(source)
 cmd=[java,'-Djava.awt.headless=true','-jar',jar,'-tsvg','-pipe','-charset','UTF-8']
 j=subprocess.run(cmd,input=source,text=True,capture_output=True,timeout=60)
 p.with_suffix('.svg').write_text(j.stdout)
 d=subprocess.run(['/tmp/rustuml-seq-recovery/group-bin/diff_one','--no-oracle',str(p)],text=True,capture_output=True,timeout=60)
 cli=subprocess.run(['/tmp/rustuml-seq-recovery/group-bin/rustuml','-tsvg','-pipe'],input=source,text=True,capture_output=True,timeout=60)
 actual=subprocess.run(['/tmp/rustuml-seq-recovery/group-bin/diff_one','--no-oracle','--print-rust',str(p)],text=True,capture_output=True,timeout=60)
 p.with_suffix('.rust.svg').write_text(cli.stdout)
 return {'source':p.name,'axes':axes,'java_exit':j.returncode,'java_stderr':j.stderr,'product_exit':d.returncode,'diff':d.stdout,'stderr':d.stderr,'cli_exit':cli.returncode,'same_compared_svg':cli.returncode==actual.returncode==0 and cli.stdout.strip()==actual.stdout.strip()}
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool: result=list(pool.map(run,rows))
(out/'results.json').write_text(json.dumps(result,indent=2)+'\n')
for r in result:
 print(r['product_exit'],r['source']);
 if r['product_exit']: print(r['diff'][:3000])
