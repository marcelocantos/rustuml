# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures,importlib.util,json,os,subprocess
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-checker');R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-9a8dea60')
spec=importlib.util.spec_from_file_location('replay',S/'replay.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);m.R=R;m.S=S
# These are unchanged existing unit-test inputs, not fresh Java comparisons.
inputs={'plain_theme':'@startuml\n!theme plain\nclass Foo\n@enduml','source_identity':'@startuml\n\n\nskinparam shadowing true\nstart\n:Fresh intake;\nif (route?) then (north)\n  :North alpha;\n  :North beta;\nelse (south)\n  :South gamma;\nendif\n:Fresh archive;\nstop\n@enduml\n'}
checks=[]
for name,source in inputs.items():
 d=S/'unit-residue'/name;d.mkdir(parents=True,exist_ok=True);(d/'input.puml').write_text(source);observations={}
 for phase in ['before','after']:
  observations[phase]={}
  for mode,args in [('svg',['-pipe']),('yaml',['--yaml','-pipe'])]:
   observations[phase][mode]=m.call([S/(phase+'-rustuml')]+args,R,dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RAYON_NUM_THREADS='4'),source.encode(),d,phase+'-'+mode)
 checks.append(dict(name=name,observations=observations,svg_byte_identical=(d/'before-svg.stdout').read_bytes()==(d/'after-svg.stdout').read_bytes(),yaml_byte_identical=(d/'before-yaml.stdout').read_bytes()==(d/'after-yaml.stdout').read_bytes()))
m.dump(S/'unit-residue.json',checks)
