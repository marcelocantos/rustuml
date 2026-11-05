# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4];SUP=Path(__file__).resolve().parent
rows=[]
for name,seconds in [('casablanca_ramadan',1710460800),('casablanca_summer',1719792000)]:
 p=Path('test-diagrams/perturbations/tim-zone-display-width')/f'checker3_{name}.puml';(ROOT/p).write_text(f'@startuml\n!$result=%date("yyyy-MM-dd z / zzzz / XXX", {seconds})\nclass "$result" as Result\n@enduml\n')
 rows.append(dict(source=str(p),mechanism='tim-zone-display-width',axes=['negative DST representation in libc versus JDK','Ramadan versus summer daylight state and display width'],zone='Africa/Casablanca',debug='date=1774210426000',kind='fresh independent supplemental'))
# First fatal must cancel later effects even when no later date error masks it.
for name,expression in [('fatal_substr_only','%substr("abc",-1)'),('fatal_unknown_only','%missing_checker3()')]:
 p=Path('test-diagrams/perturbations/tim-eager-operands-first-error')/f'checker3_{name}.puml';(ROOT/p).write_text(f'@startuml\nclass Before\n!$result={expression}\nclass After\n@enduml\n')
 rows.append(dict(source=str(p),mechanism='tim-eager-operands-first-error',axes=['first fatal builtin','later visible output cancellation'],kind='fresh isolating counterexample',negative_intent=True))
(SUP/'supplement-manifest.json').write_text(json.dumps(rows,indent=2)+'\n')
