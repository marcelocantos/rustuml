# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,os,subprocess
from pathlib import Path
root=Path('/tmp/rustuml-tim-correction')
java='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java'
jar='/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
rows=[]
for zone,debug in [('GMT+05:45','+0545'),('GMT-03:30','-0330'),('UTC','UTC')]:
 out=root/'fixed-zones'/debug;out.mkdir(parents=True,exist_ok=True)
 raw=b'@startuml\n!$s = %date("y-M-d H:m:s Z z X XX XXX", -1)\nclass "$s" as WillowClock\n@enduml\n';(out/'source.puml').write_bytes(raw)
 env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz='+debug)
 row={'java_zone':zone,'debug_zone':debug}
 for name,cmd in [('java',[java,'-Djava.awt.headless=true','-Duser.timezone='+zone,'-jar',jar,'-pipe','-tsvg']),('rust',[str(root/'fixed-rustuml'),'-pipe'])]:
  p=subprocess.run(cmd,input=raw,capture_output=True,env=env,timeout=90);(out/(name+'.svg')).write_bytes(p.stdout);(out/(name+'.stderr')).write_bytes(p.stderr);row[name+'_exit']=p.returncode
 p=subprocess.run([str(root/'probe'),'compare',str(out/'java.svg'),str(out/'rust.svg')],capture_output=True,timeout=30);(out/'comparison.txt').write_bytes(p.stdout+p.stderr);row['comparison_exit']=p.returncode;rows.append(row)
(root/'fixed-zone-results.json').write_text(json.dumps(rows,indent=2)+'\n');print(rows)
