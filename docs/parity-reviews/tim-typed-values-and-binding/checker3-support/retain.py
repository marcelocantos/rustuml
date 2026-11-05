# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json, shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4];SUP=Path(__file__).resolve().parent
maker=ROOT/'docs/parity-reviews/tim-corrective-execution-maker'
rows=[]
for r in json.loads((maker/'new-manifest.json').read_text()):
 old=maker/'inputs'/Path(r['source']).name
 p=Path('test-diagrams/perturbations')/r['mechanism']/('checker3_retained_'+old.name);(ROOT/p).parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(old,ROOT/p)
 cwd=p.parent/'checker3-maker-resources';(ROOT/cwd).mkdir(parents=True,exist_ok=True);shutil.copytree(maker/'inputs/lib',ROOT/cwd/'lib',dirs_exist_ok=True)
 rows.append(dict(r,source=str(p),cwd=str(cwd),original_source=str(old.relative_to(ROOT)),kind='retained maker, remote reference rerun',axes=['retained maker regression control','exact source and declared environment']))
for declaration,name in [('class Before','class'),('Alice -> Bob: before','sequence')]:
 for expression,kind in [('%date("QQ",0)','pattern'),('%date("yyyy",2147483648)','literal')]:
  p=Path('test-diagrams/perturbations/tim-parser-error-propagation')/f'checker3_exact_root_{name}_{kind}.puml';(ROOT/p).write_text(f'@startuml\n{declaration}\n!$bad = {expression}\n@enduml\n')
  rows.append(dict(source=str(p),mechanism='tim-parser-error-propagation',kind='exact root assertion source',axes=['class versus sequence','literal versus pattern failure'],negative=True))
(SUP/'retained-manifest.json').write_text(json.dumps(rows,indent=2)+'\n')
