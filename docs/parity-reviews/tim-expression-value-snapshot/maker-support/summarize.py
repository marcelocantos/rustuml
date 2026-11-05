# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,re,hashlib,subprocess
from pathlib import Path
s=Path('/tmp/rustuml-tim4');sup=Path('docs/parity-reviews/tim-typed-values-and-binding/checker3-support')
def summary(rows):return dict(total=len(rows),matches=sum(r['svg_match'] for r in rows),invalid=sum(r['reference_invalid'] for r in rows),cli_errors=sum(r['cli_exit']!=0 for r in rows),valid_cli_errors=sum(r['cli_exit']!=0 and not r['reference_invalid'] for r in rows),panics=sum(r['panic'] for r in rows),sources=sum(r.get('source_equal',False) for r in rows))
results={}
for suite in ['frozen','fresh','retained','supplement']:
 before={r['source']:r for r in json.loads((sup/f'after-{suite}.json').read_text())};after=json.loads((s/f'final-{suite}.json').read_text());delta={key:[] for key in ['gains','lost_passes','new_valid_errors','new_errors']}
 for r in after:
  old=before[r['source']]
  if r['svg_match'] and not old['svg_match']:delta['gains'].append(r['source'])
  if old['svg_match'] and not r['svg_match']:delta['lost_passes'].append(r['source'])
  if r['cli_exit']!=0 and old['cli_exit']==0:
   delta['new_errors'].append(r['source'])
   if not r['reference_invalid']:delta['new_valid_errors'].append(r['source'])
 results[suite]=dict(summary=summary(after),delta=delta,baseline='checker3 after-'+suite+'.json',missing=len(json.loads((s/(suite+'-missing.json')).read_text())))
 if suite=='frozen':
  for cohort in ['cohort955','cohort1044']:
   group=[r for r in after if r.get(cohort)];results[cohort]=summary(group)
(s/'deltas.json').write_text(json.dumps(results,indent=2)+'\n');print(json.dumps(results,indent=2))
for name,logname in [('builtin','oracle-final.log'),('builtin-gate','builtin-gate-final.log')]:
 log=(s/logname).read_text();tail=log.split('held-out no-oracle perturbations diverged from Java:\n',1)[1];failures=dict(re.findall(r'^  (test-diagrams/perturbations/[^\n]+?\.puml): (.*)$',tail,re.M));paths=sorted(str(p) for p in Path('test-diagrams/perturbations').rglob('*.puml'));rows=[dict(source=p,passed=p not in failures,failure=failures.get(p)) for p in paths];(s/(name+'-results.json')).write_text(json.dumps(rows,indent=2)+'\n');print(name,'matches',len(paths)-len(failures),'total',len(paths))
 allrows=sum((json.loads((s/f'final-{k}.json').read_text()) for k in ['frozen','fresh','retained','supplement']),[])
 diff=[r for r in rows if not r['passed'] and next((x['svg_match'] for x in allrows if x['source']==r['source']),False)]
 (s/(name+'-versus-isolated.json')).write_text(json.dumps(diff,indent=2)+'\n')
