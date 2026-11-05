# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,collections
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-checker');R=Path.cwd()
sha=lambda b:hashlib.sha256(b).hexdigest()
summary={};index=collections.defaultdict(list)
for suite in ['retained','maker18','sprites50']:
 rows=json.loads((S/(suite+'-membership.json')).read_bytes());before=json.loads((S/(suite+'-before.json')).read_bytes());after=json.loads((S/(suite+'-after.json')).read_bytes())
 stats=dict(entries=len(rows),distinct_pairs=len({(sha(Path(r['source_capture']).read_bytes()),sha(Path(r['golden_capture']).read_bytes())) for r in rows}),before_valid=sum(x['reference_valid'] for x in before),after_valid=sum(x['reference_valid'] for x in after),before_pass=sum(x['svg_match'] for x in before),after_pass=sum(x['svg_match'] for x in after),new_passes=[y['id'] for x,y in zip(before,after) if not x['svg_match'] and y['svg_match']],lost_passes=[y['id'] for x,y in zip(before,after) if x['svg_match'] and not y['svg_match']],changed_svg=[y['id'] for x,y in zip(before,after) if x['svg']['stdout_sha256']!=y['svg']['stdout_sha256']],changed_models=[y['id'] for x,y in zip(before,after) if x['model']!=y['model']],before_panics=sum(x['panic'] for x in before),after_panics=sum(x['panic'] for x in after),validity_disagreements=[x['id'] for x in after if not x['membership_valid_agrees']])
 summary[suite]=stats
 for r,b,a in zip(rows,before,after):index[(sha(Path(r['source_capture']).read_bytes()),sha(Path(r['golden_capture']).read_bytes()))].append(dict(id=r['id'],before=b['svg_match'],after=a['svg_match'],cwd=r['cwd']))
log=(S/'logs/standing.log').read_text();failed={x.strip().split(':',1)[0] for x in log.splitlines() if x.startswith('  test-diagrams/perturbations/')};standing=[]
for p in sorted((R/'test-diagrams/perturbations').rglob('*.puml')):
 key=(sha(p.read_bytes()),sha(p.with_suffix('.svg').read_bytes()));observations=index[key];path=str(p.relative_to(R));standing.append(dict(source=path,harness_pass=path not in failed,cli_observations=observations))
(S/'standing-mapping.json').write_text(json.dumps(standing,indent=2)+'\n')
summary['standing']=dict(entries=len(standing),harness_pass=sum(x['harness_pass'] for x in standing),cli_pass=sum(any(y['after'] for y in x['cli_observations']) for x in standing),unmapped=[x['source'] for x in standing if not x['cli_observations']],discrepancies=[x['source'] for x in standing if x['harness_pass']!=any(y['after'] for y in x['cli_observations'])])
(S/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))
