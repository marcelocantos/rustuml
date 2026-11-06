# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,collections
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();sha=lambda b:hashlib.sha256(b).hexdigest();index=collections.defaultdict(list)
for cohort in ['retained','maker18','sprites50','checker38','fresh15','fresh26','new15','delimiters4','focused2','checker30','supplement8','fresh12','checker28','fresh16','golden-regressions','checker36','fallback16']:
 rows=json.loads((S/(cohort+'-membership.json')).read_bytes());before=json.loads((S/(cohort+'-before.json')).read_bytes());after=json.loads((S/(cohort+'-after.json')).read_bytes())
 for row,b,a in zip(rows,before,after):index[(sha(Path(row['source_capture']).read_bytes()),sha(Path(row['golden_capture']).read_bytes()))].append(dict(id=row['id'],before=b['svg_match'],after=a['svg_match'],cwd=a['svg']['cwd'],env=a['env_overrides']))
failed={x.strip().split(':',1)[0] for x in (S/'logs/standing.log').read_text().splitlines() if x.startswith('  test-diagrams/perturbations/')};rows=[]
for p in sorted((R/'test-diagrams/perturbations').rglob('*.puml')):
 observations=index[(sha(p.read_bytes()),sha(p.with_suffix('.svg').read_bytes()))];path=str(p.relative_to(R));rows.append(dict(source=path,harness_pass=path not in failed,cli_observations=observations))
summary=dict(entries=len(rows),harness_pass=sum(x['harness_pass'] for x in rows),matching_env_cli_pass=sum(any(v['after'] for v in x['cli_observations']) for x in rows),before_cli_pass=sum(any(v['before'] for v in x['cli_observations']) for x in rows),unmapped=[x['source'] for x in rows if not x['cli_observations']],discrepancies=[x['source'] for x in rows if x['harness_pass']!=any(v['after'] for v in x['cli_observations'])])
(S/'standing-mapping.json').write_text(json.dumps(rows,indent=2)+'\n');(S/'standing-summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))
