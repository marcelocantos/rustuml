# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
from collections import Counter
import json,re,subprocess,hashlib
root=Path.cwd();scratch=Path('/tmp/rustuml-cff3');support=root/'docs/parity-reviews/class-command-grammar-closure/checker-support'
before=json.loads((scratch/'before/results.json').read_text());after=json.loads((scratch/'after-priority/results.json').read_text())
b={r['relative']:r for r in before['results']};a={r['relative']:r for r in after['results']}
manifest=json.loads((support/'retained-manifest.json').read_text())
sets={}
sets['current955']={r['relative'] for r in before['results'] if r['cohort']=='current955'}
sets['retained237']=set(a)-sets['current955']
sets['class23']={p for p in sets['current955'] if p.startswith('class-native-container-factory-precedence/')}
sets['sequence41']={p for p in sets['current955'] if p.startswith('sequence-grouped-note-origin/')}
sets['theme33']={p for p in sets['current955'] if p.startswith('tim-theme-post-expansion-source-identity/')}
sets['registry6']={p for p in sets['current955'] if p.startswith('render-registry-unwind-scope/')}
sets['TIM42']={p for p in sets['current955'] if p.startswith(('tim-evaluated-event-stream/','tim-explicit-date-instant/','tim-literal-variable-lookup/','tim-procedure-global-fork-memory/'))}
sets['original810']=sets['current955']-set.union(*(sets[n] for n in ['class23','sequence41','theme33','registry6','TIM42']))
sets['maker62']={str(Path(r['source']).relative_to('test-diagrams/perturbations')) for r in manifest if r['origin']=='byte-identical maker reference, independent CLI replay'}
# The checker direct sources comprise 127 initial originals + eight dependency mirrors + four final identity probes.
origins=['fresh independent checker','checker successor probe retained for dependency mapping','final independent source-guided attempt to break a narrowly passing identity mechanism']
sets['checker139']={str(Path(r['source']).relative_to('test-diagrams/perturbations')) for r in manifest if r['origin'] in origins}
sets['all1192']=set(a)
cohorts={name:dict(denominator=len(paths),before=dict(Counter(b[p]['status'] for p in paths)),after=dict(Counter(a[p]['status'] for p in paths))) for name,paths in sets.items()}
assert all(len(sets[k])==n for k,n in [('current955',955),('retained237',237),('original810',810),('maker62',62),('checker139',139)])
changed=[dict(source=p,before=b[p]['status'],after=a[p]['status']) for p in sorted(a) if b[p]['status']!=a[p]['status']]
process_errors=[dict(source=p,stderr=r['cli']['stderr']) for p,r in a.items() if r['status']=='process_error']
model=[];dropped=[]
for p in sorted(a):
 old=(Path(b[p]['output'])/'ast.txt').read_text();new=(Path(a[p]['output'])/'ast.txt').read_text()
 ids=lambda text:set(re.findall(r'^\s+ClassEntity \{\n\s+id: (".*"),',text,re.M))
 counts=lambda text:{k:len(re.findall(r'^\s+'+k+r' \{\s*$',text,re.M)) for k in ['ClassEntity','Package','Note','Relationship']}
 entry=dict(source=p,before_family=old.split('(')[0].strip(),after_family=new.split('(')[0].strip(),before=counts(old),after=counts(new),removed_class_ids=sorted(ids(old)-ids(new)))
 model.append(entry)
 if a[p]['status']!='invalid_reference' and old.startswith('Class(') and new.startswith('Class(') and (entry['removed_class_ids'] or any(entry['after'][k]<entry['before'][k] for k in ['Package','Note','Relationship'])):dropped.append(entry)
previous={r['relative']:r for r in json.loads((support/'current-results.json').read_text())['results']}
cwd_differences=[dict(source=p,checker=previous[p]['status'],own_before=b[p]['status']) for p in sets['current955'] if previous[p]['status']!=b[p]['status']]
summary=dict(status='maker evidence; not an acceptance',tested_revision=after['tested_revision'],cohorts=cohorts,changed_cases=changed,lost_passes=[r for r in changed if r['before']=='pass'],new_process_errors=[r for r in changed if r['after']=='process_error' and r['before']!='invalid_reference'],remaining_process_errors=process_errors,retained_checker_residuals=[p for p in sorted(sets['retained237']) if a[p]['status']=='mismatch'],same_class_model_losses=dropped,cwd_differences=cwd_differences,parent_inventory=dict(latest_reported=1449,own_frozen=1192,not_replayed=257,original_frozen=955,duplicates_explicit=True),main_tier_before=dict(source_only=[11188,11251],assisted=[10497,11251],provenance='Independent checker REPORT at 502960ac; not represented as another full maker baseline run. Own before CLI replays all 1192 and the union of 77 main failure candidates.'))
(scratch/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');(scratch/'model-inventory.json').write_text(json.dumps(model,indent=2)+'\n')
print(json.dumps(cohorts,indent=2));print('model losses',len(dropped),'cwd',cwd_differences)
