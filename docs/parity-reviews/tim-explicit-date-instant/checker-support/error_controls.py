# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Preserve deliberately invalid date calls; never count them as valid positives."""
import importlib.util
import json
from pathlib import Path

path=Path.cwd()/'docs/parity-reviews/tim-evaluated-event-stream/checker-support/check.py'
spec=importlib.util.spec_from_file_location('check',path)
m=importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
manifest=json.loads((m.SUPPORT/'case-manifest.json').read_text())
rows=json.loads((m.SUPPORT/'fresh-results.json').read_text())
for name,expression in [('overflow_seconds','%date("yyyy", 2147483648)'),('bad_pattern','%date("QQ", 0)')]:
    source=Path('test-diagrams/perturbations/tim-explicit-date-instant')/('checker_invalid_'+name+'.puml')
    source.write_text('@startuml\n!$stamp = '+expression+'\nclass "$stamp" as InvalidClock\n@enduml\n')
    meta={'axes':['explicit date argument error','Java rejection versus silent Rust output'],'kind':'deliberately invalid checker control'}
    manifest[str(source)]=meta
    row=m.measure(source,True,meta)
    rows.append(row)
    print(json.dumps(row))
(m.SUPPORT/'case-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
(m.SUPPORT/'fresh-results.json').write_text(json.dumps(rows,indent=2)+'\n')
