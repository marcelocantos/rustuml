# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json
import os
from pathlib import Path
import subprocess

root = Path('/tmp/rustuml-tim-checker')
data = json.loads((root/'cases.json').read_text())
java = '/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java'
jar = '/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
new = {
 'checker_fixed_timestamp': ('!theme cerulean\n!$stamp = %date("yyyy", 946684800)\nclass "Clock $stamp" as FixedClockLedger', ['explicit timestamp', 'theme variable source identity']),
 'checker_inner_comment': ('!theme cerulean\nclass BirchLedger /\' harmless \'/', ['inline block comment', 'themed entity']),
}
for name, (body, axes) in new.items():
 source = '@startuml\n'+body+'\n@enduml\n'
 directory = root/'cases'/name
 directory.mkdir(exist_ok=True)
 path = directory/'source.puml'
 path.write_text(source)
 p = subprocess.run([java, '-Djava.awt.headless=true', '-jar', jar, '-pipe', '-tsvg'], input=source.encode(), capture_output=True, timeout=90)
 (directory/'java.svg').write_bytes(p.stdout)
 (directory/'java.stderr').write_bytes(p.stderr)
 q = subprocess.run([java, '-Djava.awt.headless=true', '-jar', jar, '-preproc', str(path)], capture_output=True, timeout=90)
 (directory/'preproc.stderr').write_bytes(q.stderr)
 path.with_suffix('.preproc').rename(directory/'java.expanded')
 data[name] = {'source': source,'axes':axes,'java_exit':p.returncode,'preproc_exit':q.returncode}
 # Already-built baseline binary and probe remain the baseline product paths.
 env = dict(os.environ, PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib', RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
 r = {'source':str(path)}
 p = subprocess.run([str(root/'before-rustuml'), '-pipe'], input=source.encode(), capture_output=True, env=env, timeout=90)
 (directory/'before.svg').write_bytes(p.stdout)
 r['cli_exit']=p.returncode
 r['stderr']=p.stderr.decode()
 p = subprocess.run([str(root/'before-probe'),'compare',str(directory/'java.svg'),str(directory/'before.svg')],capture_output=True,timeout=30)
 r['comparison_exit']=p.returncode
 r['comparison']=(p.stdout+p.stderr).decode()
 r['result']='pass' if p.returncode==0 else 'fail'
 for mode in ['expand','lines']:
  p=subprocess.run([str(root/'before-probe'),mode,str(path)],capture_output=True,env=env,timeout=30)
  (directory/f'before.{mode}').write_bytes(p.stdout)
  r[mode+'_exit']=p.returncode
 r['expansion_equal']=(directory/'before.expand').read_bytes()==(directory/'java.expanded').read_bytes()
 results=json.loads((root/'before-fresh.json').read_text())
 results.append(r)
 (root/'before-fresh.json').write_text(json.dumps(results,indent=2)+'\n')
(root/'cases.json').write_text(json.dumps(data,indent=2)+'\n')
