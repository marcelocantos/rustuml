import concurrent.futures,json,subprocess
from pathlib import Path
out=Path('/tmp/rustuml-seq-recovery/fresh-envelope');out.mkdir(exist_ok=True)
java='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java';jar='/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
rows=[]
for position in (0,1,2):
 for pad in (0,3):
  for depth in (0,1,3):
   for label in ('tiny','Wide nested WWWWWWWW.ff'):
    names=['L','R'];names.insert(position,'S')
    body=f'skinparam Padding {pad}\n'+''.join('participant '+n+'\n' for n in names)+'group scope\nL -> S : seed\n'+''.join('S -> S ++ : recursive\n' for _ in range(depth))+'S -> S : '+label+'\n'+''.join('return unwind\n' for _ in range(depth))+'end\n'
    rows.append((f'envelope_{len(rows)}',body))
for mode in ('create','destroy','note','newpage'):
 for depth in (1,3):
  body='participant L\n'+('create S\nL -> S ++ : construct\n' if mode=='create' else 'participant S\nL -> S : enter\n')
  if mode=='newpage':body+='newpage Continuation\n'
  body+=''.join('S -> S ++ : nested\n' for _ in range(depth))+'S -> S : Wide WWWWWWWWWW\n'
  if mode=='note':body+='note right : held note\n'
  body+=''.join('return done\n' for _ in range(depth))
  if mode=='destroy':body+='L -> S !! : destroy\n'
  rows.append((f'envelope_{len(rows)}',body))
def run(row):
 name,body=row;p=out/(name+'.puml');src='@startuml\n'+body+'@enduml\n';p.write_text(src)
 j=subprocess.run([java,'-Djava.awt.headless=true','-jar',jar,'-tsvg','-pipe','-charset','UTF-8'],input=src,text=True,capture_output=True,timeout=60);p.with_suffix('.svg').write_text(j.stdout)
 return {'source':p.name,'java_exit':j.returncode,'java_stderr':j.stderr}
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool: results=list(pool.map(run,rows))
(out/'generated.json').write_text(json.dumps(results,indent=2)+'\n');print(len(results),'cases',sum(r['java_exit']!=0 for r in results),'Java errors')
