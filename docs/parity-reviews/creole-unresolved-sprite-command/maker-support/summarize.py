# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,re,hashlib
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-maker');R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-a295df45')
sha=lambda b:hashlib.sha256(b).hexdigest()
def load(n):return json.loads((S/n).read_bytes())
def dump(n,v):(S/n).write_text(json.dumps(v,indent=2)+'\n')
rows=load('membership.json')+load('checker8-membership.json');phases={p:load(p+'-results.json')+load('checker8-'+p+'-results.json') for p in ['before','after']}
cohorts={}
for suite in dict.fromkeys(r['suite'] for r in rows):
 indexes=[i for i,r in enumerate(rows) if r['suite']==suite];cohorts[suite]=dict(entries=len(indexes))
 for phase,values in phases.items():
  v=[values[i] for i in indexes];cohorts[suite][phase]=dict(valid=sum(x['reference_valid'] for x in v),svg=sum(x['svg_match'] for x in v),panics=sum(x['panic'] for x in v),cli_errors=sum(x['svg']['exit']!=0 for x in v),available_source=sum('exact_source' in x for x in v),exact_source=sum(x.get('exact_source',False) for x in v))
losses=[a['id'] for a,b in zip(phases['before'],phases['after']) if a['svg_match'] and not b['svg_match']]
gains=[a['id'] for a,b in zip(phases['before'],phases['after']) if not a['svg_match'] and b['svg_match']]
model_changes=[a['id'] for a,b in zip(phases['before'],phases['after']) if a.get('model')!=b.get('model')]
lookup={}
for row,before,after in zip(rows,phases['before'],phases['after']):
 key=(sha(Path(row['source_capture']).read_bytes()),sha(Path(row['golden_capture']).read_bytes()));lookup.setdefault(key,[]).append(dict(id=row['id'],before=before['svg_match'],after=after['svg_match'],cwd=row['cwd']))
standing_text=(S/'logs/standing.log').read_text();fails=set(re.findall(r'^  (test-diagrams/perturbations/[^\n]*?\.puml):',standing_text,re.M));standing=[]
for p in sorted((R/'test-diagrams/perturbations').rglob('*.puml')):
 rel=str(p.relative_to(R));key=(sha(p.read_bytes()),sha(p.with_suffix('.svg').read_bytes())); matches=lookup.get(key,[])
 standing.append(dict(source=rel,harness_pass=rel not in fails,cli_observations=matches))
assert not [x for x in standing if not x['cli_observations']], 'unmapped standing pair'
dump('standing-mapping.json',standing)
fb=load('fresh-before-results.json');fa=load('fresh-after-results.json');fresh=[]
for a,b in zip(fb,fa):
 assert a['row']==b['row'];fresh.append(dict(name=Path(a['row']['source']).stem,classification=b['classification'],before=a['phases']['before']['svg_match'],after=b['phases']['after']['svg_match'],source_equal=b['direct_source_equal'],cli_source_equal=b['phases']['after'].get('exact_source'),java_svg_exit=b['java_svg_exit'],rust_svg_exit=b['phases']['after']['svg']['exit']))
summary=dict(retained_entries=len(rows),retained_distinct_pairs=len(lookup),retained_duplicates=len(rows)-len(lookup),cohorts=cohorts,losses=losses,gains=gains,model_changes=model_changes,retained={p:dict(valid=sum(x['reference_valid'] for x in v),svg=sum(x['svg_match'] for x in v),panics=sum(x['panic'] for x in v)) for p,v in phases.items()},fresh=fresh,standing=dict(entries=len(standing),harness_pass=sum(x['harness_pass'] for x in standing),cli_pass=sum(any(y['after'] for y in x['cli_observations']) for x in standing),differences=[x['source'] for x in standing if x['harness_pass']!=any(y['after'] for y in x['cli_observations'])]),main_tiers=dict(kind='library diagnostics, not actual CLI headline metrics',product='11179/11251',assisted='10497/11251',exit=101))
dump('summary.json',summary)
print(json.dumps({k:v for k,v in summary.items() if k not in ['cohorts','fresh']},indent=2))
