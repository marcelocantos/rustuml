# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,json,re
from pathlib import Path
R=Path.cwd();S=Path('/tmp/rustuml-tim5');rows=json.loads((S/'iteration1-collateral.json').read_bytes());fresh=json.loads((S/'iteration1-fresh31.json').read_bytes())
def count(rs):return dict(total=len(rs),matches=sum(r['svg_match'] for r in rs),invalid=sum(r.get('reference_invalid',not r.get('reference_valid',True)) for r in rs),cli_errors=sum(r['cli_exit']!=0 for r in rs),valid_cli_errors=sum(r['cli_exit']!=0 and not r.get('reference_invalid',not r.get('reference_valid',True)) for r in rs),panics=sum(r['panic'] for r in rs))
summary=dict(collateral=count(rows),fresh31=count(fresh),union1547=count(rows+fresh),cohorts={k:count([r for r in rows if r.get(k)]) for k in ['cohort955','cohort1044']})
summary['parent1449']=count([r for r in rows if r['cohort']=='frozen'])
log=(S/'perturbations.log').read_text().split('held-out no-oracle perturbations diverged from Java:\n',1)[1]
failures=dict(re.findall(r'^  (test-diagrams/perturbations/[^\n]+?\.puml): (.*)$',log,re.M));paths=sorted(str(p.relative_to(R)) for p in (R/'test-diagrams/perturbations').rglob('*.puml'))
assert len(paths)==1547
standing=[dict(source=p,passed=p not in failures,failure=failures.get(p)) for p in paths];by={r['source']:r for r in rows+fresh}
diffs=[dict(r,isolated=by[r['source']]['svg_match']) for r in standing if r['passed']!=by[r['source']]['svg_match']]
summary['standing']=dict(total=len(paths),matches=sum(r['passed'] for r in standing),different_from_matching_environment_cli=len(diffs))
for name,data in [('summary',summary),('standing-results',standing),('standing-versus-isolated',diffs)]: (S/(name+'.json')).write_text(json.dumps(data,indent=2)+'\n')
print(json.dumps(summary,indent=2))
