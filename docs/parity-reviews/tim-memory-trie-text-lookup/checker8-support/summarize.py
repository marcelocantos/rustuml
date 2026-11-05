# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections
import json
import re
import subprocess
from pathlib import Path
from replay import R, S, SUITES, dump, sha, call

rows=json.loads((S/'membership.json').read_bytes())
phases={p:{r['id']:r for r in json.loads((S/(p+'-results.json')).read_bytes())} for p in ['before','after']}
summary={};sets={};losses=[];gains=[];errors=[];drops=[];panics=collections.defaultdict(list);nonpositive=[];classification_disagreements=[]
for suite in SUITES+['maker13','maker2']:
    members=[r for r in rows if r['suite']==suite];sets[suite]={(r['source_sha256'],r['golden_sha256']) for r in members}
    d=dict(entries=len(members),distinct_pairs=len(sets[suite]))
    for p in phases:
        vals=[phases[p][r['id']] for r in members]
        d[p]=dict(valid=sum(v['reference_valid'] for v in vals),svg=sum(v['svg_match'] for v in vals),
                  valid_source_available=sum(v['reference_valid'] and 'reference_source_capture' in r for r,v in zip(members,vals)),
                  valid_source_exact=sum(v['reference_valid'] and v.get('exact_source',False) for v in vals),
                  all_source_exact=sum(v.get('exact_source',False) for v in vals),panics=sum(v['panic'] for v in vals))
    summary[suite]=d
for row in rows:
    a,b=phases['after'][row['id']],phases['before'][row['id']]
    if b['svg_match'] and not a['svg_match']: losses.append(row['id'])
    if not b['svg_match'] and a['svg_match']: gains.append(row['id'])
    if a['reference_valid'] and not b['svg']['exit'] and a['svg']['exit']: errors.append(row['id'])
    if not a['membership_valid_agrees']: classification_disagreements.append(row['id'])
    if (b.get('model') or {}).get('entities') is not None and ((a.get('model') or {}).get('entities') or 0)<b['model']['entities']:
        drops.append(dict(id=row['id'],reference_valid=a['reference_valid'],before=b['model'],after=a.get('model')))
    err=(S/'replay/after'/row['id']/'svg.stderr').read_text()
    if a['panic']:
        signature=err[err.index('panicked at '):].split('\nnote:',1)[0].strip();panics[signature].append(row['id'])
    if not a['reference_valid']:
        nonpositive.append(dict(id=row['id'],original_membership=row['membership'],original_classification=row['original_classification'],
                                cli_exit=a['svg']['exit'],comparator_exit=a['compare']['exit'],welcome_text_present=a['reference_welcome'],
                                factory=a['reference_factory'],stderr=err,cli_source_exact=a.get('exact_source')))
# Map frozen standing membership by exact bytes, keeping one executable path per source.
pair_to_rows=collections.defaultdict(list)
for row in rows: pair_to_rows[(row['source_sha256'],row['golden_sha256'])].append(row)
standing=[];fails={}
for line in (S/'logs/standing.log').read_text().splitlines():
    if line.startswith('  test-diagrams/perturbations/'):
        path,reason=line[2:].split(': ',1);fails[path]=reason
for p in sorted((R/'test-diagrams/perturbations').rglob('*.puml')):
    rel=str(p.relative_to(R)); pair=(sha(p.read_bytes()),sha(p.with_suffix('.svg').read_bytes()))
    matched=pair_to_rows[pair];assert matched,rel
    # Duplicates have equal CLI pass outcomes, despite independently captured processes.
    values={phases['after'][r['id']]['svg_match'] for r in matched};assert len(values)==1
    cli=next(iter(values));standing.append(dict(source=rel,standing_pass=rel not in fails,cli_pass=cli,
                                              matched_ids=[r['id'] for r in matched],failure=fails.get(rel)))
assert len(standing)==1628
different=[r for r in standing if r['standing_pass']!=r['cli_pass']]
retained=[r for r in rows if r['suite'] not in ['fresh23','maker13','maker2']]
retained_stats=dict(entries=len(retained),**{p:dict(valid=sum(phases[p][r['id']]['reference_valid'] for r in retained),
    svg=sum(phases[p][r['id']]['svg_match'] for r in retained),
    exact_source=sum(phases[p][r['id']]['reference_valid'] and phases[p][r['id']].get('exact_source',False) for r in retained),
    available_source=sum(phases[p][r['id']]['reference_valid'] and 'reference_source_capture' in r for r in retained)) for p in phases})
result=dict(entries=len(rows),distinct_pairs=len(set.union(*sets.values())),duplicate_entries=len(rows)-len(set.union(*sets.values())),
            cohort_summary=summary,retained=retained_stats,intersections={a+' & '+b:len(sets[a]&sets[b]) for i,a in enumerate(sets) for b in list(sets)[i+1:]},
            lost_valid_passes=losses,new_valid_errors=errors,gains=gains,model_drops=drops,valid_model_drops=[r for r in drops if r['reference_valid']],
            panic_groups=dict(panics),classification_disagreements=classification_disagreements,
            standing=dict(total=len(standing),passed=sum(r['standing_pass'] for r in standing),matching_environment_cli=sum(r['cli_pass'] for r in standing),discrepancies=different),
            known_valid_errors=[dict(id=r['id'],panic=phases['after'][r['id']]['panic']) for r in rows if phases['after'][r['id']]['reference_valid'] and phases['after'][r['id']]['svg']['exit']],
            valid_source_mismatches=[r['id'] for r in rows if phases['after'][r['id']]['reference_valid'] and 'reference_source_capture' in r and not phases['after'][r['id']].get('exact_source',False)])
dump(S/'summary.json',result);dump(S/'nonpositive-statuses.json',nonpositive);dump(S/'standing-mapping.json',standing)
print(json.dumps({k:v for k,v in result.items() if k in ['entries','distinct_pairs','duplicate_entries','retained','lost_valid_passes','new_valid_errors','valid_model_drops','valid_source_mismatches']},indent=2))
print('standing',len(standing),sum(r['standing_pass'] for r in standing),'CLI',sum(r['cli_pass'] for r in standing),'discrepancies',len(different))
