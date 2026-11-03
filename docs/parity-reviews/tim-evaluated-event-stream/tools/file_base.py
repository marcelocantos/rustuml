# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
import json,os,subprocess,yaml
root=Path('/tmp/rustuml-tim-repair');folder=root/'file-base';folder.mkdir(exist_ok=True)
(folder/'settings.iuml').write_text('!$shade = "#915237"\n!theme cerulean\nskinparam classBorderColor $shade\n')
source=folder/'source.puml';source.write_text('@startuml\n!include settings.iuml\nclass AlexandriteLedger\n@enduml\n')
java='/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java';jar='/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
probe=(root/'SourceProbe.java').read_text().replace('import net.sourceforge.plantuml.SourceStringReader;','import net.sourceforge.plantuml.SourceStringReader;\nimport net.sourceforge.plantuml.security.SFile;').replace('new SourceStringReader(source);','args.length > 1 ? new SourceStringReader(source, new SFile(args[1])) : new SourceStringReader(source);')
(root/'SourceProbe.java').write_text(probe)
subprocess.run([str(Path(java).with_name('javac')),'-cp',jar,str(root/'SourceProbe.java')],check=True)
p=subprocess.run([java,'-Djava.awt.headless=true','-jar',jar,'-pipe','-tsvg'],input=source.read_bytes(),capture_output=True,cwd=folder,timeout=90)
(folder/'java.svg').write_bytes(p.stdout);(folder/'java.stderr').write_bytes(p.stderr);result={'java_svg_exit':p.returncode}
p=subprocess.run([java,'-Djava.awt.headless=true','-cp',str(root)+':'+jar,'SourceProbe',str(source),str(folder)],capture_output=True,timeout=90)
(folder/'java.uml-source').write_bytes(p.stdout);(folder/'java.source.stderr').write_bytes(p.stderr);result['java_source_exit']=p.returncode
for phase in ['before','candidate']:
 env=dict(os.environ,PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
 p=subprocess.run([str(root/f'{phase}-rustuml'),str(source)],capture_output=True,env=env,timeout=90)
 (folder/f'{phase}.svg').write_bytes(p.stdout);(folder/f'{phase}.stderr').write_bytes(p.stderr);result[phase+'_cli_exit']=p.returncode
 p=subprocess.run([str(root/'probe'),'compare',str(folder/'java.svg'),str(folder/f'{phase}.svg')],capture_output=True,timeout=90)
 (folder/f'{phase}.diff.txt').write_bytes(p.stdout+p.stderr);result[phase+'_comparison_exit']=p.returncode
 p=subprocess.run([str(root/f'{phase}-rustuml'),'--yaml',str(source)],capture_output=True,env=env,timeout=90)
 (folder/f'{phase}.yaml').write_bytes(p.stdout);node=yaml.compose(p.stdout)
 for key in ['diagram','meta','source']:node={k.value:v for k,v in node.value}[key]
 (folder/f'{phase}.uml-source').write_text(node.value);result[phase+'_source_equal']=node.value.encode()==(folder/'java.uml-source').read_bytes()
(root/'file-base-results.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
