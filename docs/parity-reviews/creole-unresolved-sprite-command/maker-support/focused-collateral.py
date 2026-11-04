# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures,importlib.util,json,os,subprocess
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-maker');R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-a295df45')
spec=importlib.util.spec_from_file_location('replay',S/'replay.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
rows=[]
for i,p in enumerate(sorted((R/'test-diagrams/golden/sprites').rglob('*.puml'))):
 q=subprocess.run([S/'probe','compare',p.with_suffix('.svg'),p.with_suffix('.svg')],capture_output=True)
 rows.append(dict(id='sprites/'+p.stem,suite='sprites',index=i,membership=dict(reference_valid=q.returncode==0),cwd=str(R),source_capture=str(p),golden_capture=str(p.with_suffix('.svg'))))
m.dump(S/'sprites-membership.json',rows)
for phase in ['before','after']:
 with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:out=list(pool.map(lambda r:m.measure(r,phase),rows))
 m.dump(S/('sprites-'+phase+'-results.json'),out);print(phase,len(out),sum(x['reference_valid'] for x in out),sum(x['svg_match'] for x in out))
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
