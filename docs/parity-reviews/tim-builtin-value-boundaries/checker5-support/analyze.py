# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,gzip,hashlib,json,re,subprocess
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();sha=lambda b:hashlib.sha256(b).hexdigest()
def load(p):return json.loads(gzip.decompress(p.read_bytes()) if p.suffix=='.gz' else p.read_bytes())
rows=load(S/'collateral-results.json')+load(S/'prior31-results.json');by={r['source']:r for r in rows}
failures={}
for line in (S/'logs/standing.log').read_text().splitlines():
 if line.startswith('  test-diagrams/perturbations/'):
  p,msg=line[2:].split(': ',1);failures[p]=msg
assert len(by)==1547
changes=[dict(source=p,standing_pass=p not in failures,cli_pass=r['phases']['after']['svg_match'],standing_failure=failures.get(p),cwd=r.get('cwd','.'),zone=r.get('zone','GMT+11:00'),debug=r.get('debug','date=1774210426000,tz=AEDT+1100'),env=r.get('env',{})) for p,r in by.items() if (p not in failures)!=r['phases']['after']['svg_match']]
entity_drops=[];source_models=[];panics=collections.defaultdict(list);other_errors=[]
for r in rows:
 a=r['phases']['after'];b=r['phases']['before']
 if a.get('model') and b.get('model'):
  x,y=b['model'].get('entities'),a['model'].get('entities');drop=x is not None and y is not None and y<x
  if drop:entity_drops.append(r['source'])
  source_models.append(dict(source=r['source'],before=b['model'],after=a['model'],entity_drop=drop))
 if a['panic']:
  suite='prior31' if r in rows[1516:] else 'collateral';err=(S/'replay'/suite/'after'/r['capture_id']/'svg.stderr').read_text();location=err[err.index('panicked at '):].split('\nnote:',1)[0].strip();panics[location].append(r['source'])
 elif r['reference_valid'] and a['cli_exit']:other_errors.append(r['source'])
summary=dict(frozen1449=dict(total=sum(r.get('cohort')=='frozen' for r in rows),matches=sum(r['phases']['after']['svg_match'] for r in rows if r.get('cohort')=='frozen')),standing=dict(total=1547,passed=1547-len(failures),failed=len(failures),exit=101),matching_env_cli=sum(r['phases']['after']['svg_match'] for r in rows),standing_cli_differences=changes,entity_drops=entity_drops,panic_groups=dict(panics),other_valid_errors=other_errors)
(S/'analysis.json').write_text(json.dumps(summary,indent=2)+'\n');(S/'source-models.json').write_text(json.dumps(source_models,indent=2)+'\n');print('summary',summary['frozen1449'],summary['standing'],summary['matching_env_cli'],'differences',len(changes),'model drops',len(entity_drops),'panic groups',[(k,len(v)) for k,v in panics.items()],'other',other_errors)
# Ensure the exact historical auxiliary sources/references retain byte provenance.
verified=[]
for cohort,base,relay in [('maker15',R/'docs/parity-reviews/tim-builtin-value-boundaries/maker-support','relay15'),('maker17',R/'docs/parity-reviews/tim-expression-value-snapshot/maker-support','relay-completion')]:
 for r in load(base/relay/'results.json'):
  p=base/'inputs'/Path(r['source']).name;assert sha(p.read_bytes())==r['source_sha256'];verified.append(dict(cohort=cohort,source=str(p.relative_to(R)),sha256=sha(p.read_bytes())))
(S/'auxiliary-integrity.json').write_text(json.dumps(verified,indent=2)+'\n')
