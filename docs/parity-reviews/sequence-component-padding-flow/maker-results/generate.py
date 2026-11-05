import concurrent.futures,json,subprocess
from pathlib import Path
out=Path('/tmp/rustuml-seq-recovery/fresh-padding');out.mkdir(exist_ok=True)
java='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java';jar='/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
rows=[]
for pad in (0,2,5,9):
 for header in ('group Package','alt allowed','alt first\\nsecond'):
  for position in ('left of F','right of F','over F','right'):
   body=f'skinparam Padding {pad}\nparticipant Seed as S\nparticipant Forge as F\n{header}\nS -> F ++ : acquire\nnote {position}\nWide Wi.f\nsecond line\nend note\nopt verify\nnote left of S : MMM short\nreturn answer\nend\n'
   if header.startswith('alt'): body+='else fallback\\nmore\nS -> F : finish\n'
   body+='end\n'
   rows.append((f'padding_{len(rows)}',body,['padding','group/comment structure','note position and attachment']))
def run(row):
 name,body,axes=row;src='@startuml\n'+body+'@enduml\n';p=out/(name+'.puml');p.write_text(src)
 j=subprocess.run([java,'-Djava.awt.headless=true','-jar',jar,'-tsvg','-pipe','-charset','UTF-8'],input=src,text=True,capture_output=True,timeout=60);p.with_suffix('.svg').write_text(j.stdout)
 return {'source':p.name,'axes':axes,'java_exit':j.returncode,'java_stderr':j.stderr}
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool: rows=list(pool.map(run,rows))
(out/'generated.json').write_text(json.dumps(rows,indent=2)+'\n');print(len(rows),'cases',sum(r['java_exit']!=0 for r in rows),'Java errors')
