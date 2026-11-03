# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures,json,os,subprocess
from pathlib import Path
root=Path('/tmp/rustuml-tim-correction')
java='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java'
jar='/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
sources=sorted(Path('test-diagrams/perturbations/tim-explicit-date-instant').glob('checker_dst_*.puml'))
def run(case):
 zone,source=case;out=root/'timezones'/zone.replace('/','_')/source.stem;out.mkdir(parents=True,exist_ok=True)
 raw=source.read_bytes();(out/'source.puml').write_bytes(raw)
 env=dict(os.environ,TZ=zone,RUSTUML_DEBUG='date=1774210426000',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
 commands={'java':[java,'-Djava.awt.headless=true','-Duser.timezone='+zone,'-jar',jar,'-pipe','-tsvg'],'rust':[str(root/'fixed-rustuml'),'-pipe']};row={'source':str(source),'zone':zone,'environment':{'TZ':zone,'RUSTUML_DEBUG':env['RUSTUML_DEBUG']}}
 for name,cmd in commands.items():
  p=subprocess.run(cmd,input=raw,capture_output=True,env=env,timeout=90);(out/(name+'.svg')).write_bytes(p.stdout);(out/(name+'.stderr')).write_bytes(p.stderr);row[name+'_exit']=p.returncode
 p=subprocess.run([str(root/'probe'),'compare',str(out/'java.svg'),str(out/'rust.svg')],capture_output=True,timeout=30);(out/'comparison.txt').write_bytes(p.stdout+p.stderr);row['comparison_exit']=p.returncode
 return row
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(run,[(zone,source) for zone in ['Australia/Melbourne','Asia/Kathmandu','UTC'] for source in sources]))
(root/'timezone-results.json').write_text(json.dumps(rows,indent=2)+'\n');print('Timezone controls',len(rows),'passing',sum(r['java_exit']==r['rust_exit']==r['comparison_exit']==0 for r in rows))
