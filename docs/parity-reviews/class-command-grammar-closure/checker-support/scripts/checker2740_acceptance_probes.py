# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Additional source-guided attempts to break the narrowly passing identity models."""
from pathlib import Path
import json
import runpy
g = runpy.run_path('/tmp/checker2740_generate.py')
g['CASES'].clear()
add = g['add']
add('symbol-container-alias-identity','dotted_alias','node "Wide as [component]" as Remote.Group {\nclass First\n}\nnode Remote.Group {\nclass Second\nFirst --> Second\n}', 'dotted alias and command-looking quoted display', 'reopened container with child relationship')
add('symbol-container-alias-identity','nested_opaque','left to right direction\nframe Harbor {\ncloud "component as [actor]" as G {\nclass LongPayload\n}\ncloud G {\nclass X\nX --> LongPayload\n}\n}', 'nested opaque display and repeated alias', 'LR direction and reversed child declaration/link order')
add('quoted-code-identity','quoted_dotted','package Container {\nclass "A dotted.name"\nclass Peer\n"A dotted.name" --> Peer\nnote left of "A dotted.name" : retained\n}', 'literal dot plus space in quoted code', 'scoped link and left note resolution')
add('quoted-code-identity','quoted_lr_order','left to right direction\nclass Peer\nclass "node as [Class]"\nPeer --> "node as [Class]"\nnote bottom of "node as [Class]" : retained', 'quoted brackets and as keyword at root', 'LR direction and peer declared first')
results=[g['generate'](c) for c in g['CASES']]
(g['OUT']/'generation-acceptance.json').write_text(json.dumps(results,indent=2)+'\n')
r = runpy.run_path('/tmp/checker2740_run.py')
rows=[r['run_one'](('acceptance_probes',Path(c['source']),Path(c['source']).relative_to(g['SOURCES']))) for c in results]
(g['OUT']/'acceptance-results.json').write_text(json.dumps({'results':rows},indent=2)+'\n')
for row in rows:print(row['relative'],row['status'])
