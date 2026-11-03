# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures
import json
import os
from pathlib import Path
import subprocess
import yaml

root=Path('/tmp/rustuml-tim-checker')
java='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java'
jar='/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
subprocess.run([str(Path(java).with_name('javac')),'-cp',jar,str(root/'SourceProbe.java')],check=True)
env=dict(os.environ,PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
cases=json.loads((root/'cases.json').read_text())
def measure(name):
 directory=root/'cases'/name
 source=directory/'source.puml'
 p=subprocess.run([java,'-Djava.awt.headless=true','-cp',str(root)+':'+jar,'SourceProbe',str(source)],capture_output=True,timeout=90)
 (directory/'java.uml-source').write_bytes(p.stdout)
 (directory/'java.uml-source.stderr').write_bytes(p.stderr)
 row={'name':name,'java_exit':p.returncode}
 for phase in ['before','after']:
  p=subprocess.run([str(root/f'{phase}-rustuml'),'--yaml','-pipe'],input=source.read_bytes(),capture_output=True,env=env,timeout=90)
  row[phase+'_exit']=p.returncode
  node=yaml.compose(p.stdout)
  for key in ['diagram','meta','source']:
   node={k.value:v for k,v in node.value}[key]
  text=node.value
  (directory/f'{phase}.uml-source').write_text(text)
  row[phase+'_equal']=text.encode()==(directory/'java.uml-source').read_bytes()
 return row
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
 results=list(pool.map(measure,cases))
(root/'metadata.json').write_text(json.dumps(results,indent=2)+'\n')
print(json.dumps(results,indent=2))
