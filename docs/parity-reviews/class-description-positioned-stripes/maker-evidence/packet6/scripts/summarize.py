# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,re,shutil,subprocess
from collections import Counter
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-class-cff6');P=Path('/Users/marcelo/Library/Caches/rustuml-class-cff5')
a=json.load(open(S/'final1649.json'));after={r['source']:r for r in a['results']};base=json.load(open(P/'inventory-v1.json'))['results'];base+=json.load(open(P/'focused-v1.json'))['results'];f=json.load(open(P/'fresh-comparison.json'))['results'];paths=json.load(open(S/'old56.json'))[36:]
byname={r['name']:r for r in f}
for path in paths:
 r=byname[Path(path).name];base.append({'source':path,'status':r['after']['status'],'source_sha256':r['source_sha256']})
base+=json.load(open(S/'before34.json'))['results'];before={r['source']:r for r in base};assert set(before)==set(after)
oldcoh=json.load(open('docs/parity-reviews/class-note-quark-modifier-lifecycle/maker-evidence/recovery4/corrected-cohort-membership.json'))
cohorts=oldcoh|{'maker36':json.load(open(S/'old56.json'))[:36],'maker20':paths,'checker34':json.load(open(S/'review34.json')),'all1649':json.load(open(S/'all1649.json'))}
report={k:{'entries':len(ps),'before':dict(Counter(before[p]['status'] for p in ps)),'after':dict(Counter(after[p]['status'] for p in ps))} for k,ps in cohorts.items()}
f26=json.load(open(S/'final26.json'));b26=json.load(open(S/'before26.json'));report['fresh26']={'entries':26,'before':b26['counts'],'after':f26['counts']}
sets={k:{after[p]['source_sha256'] for p in ps} for k,ps in [('preserved1559',json.load(open(S/'all1649.json'))[:1559]),('maker36',cohorts['maker36']),('maker20',paths),('checker34',cohorts['checker34'])]};sets['fresh26']={r['source_sha256'] for r in f26['results']}
report['overlaps']={'unique_hashes':{k:len(v) for k,v in sets.items()},'pairwise_shared':{f'{k}/{l}':len(v&sets[l]) for i,(k,v) in enumerate(sets.items()) for l in list(sets)[i+1:]},'all1675_unique_source_hashes':len(set.union(*sets.values()))}
(S/'cohorts.json').write_text(json.dumps(report,indent=2)+'\n')
delta={'repaired':[p for p in after if before[p]['status']!='pass' and after[p]['status']=='pass'],'lost_passes':[p for p in after if before[p]['status']=='pass' and after[p]['status']!='pass'],'new_errors':[p for p in after if before[p]['status']!='process_error' and after[p]['status']=='process_error'],'repaired_errors':[p for p in after if before[p]['status']=='process_error' and after[p]['status']!='process_error'],'residuals':[{'source':p,'before':before[p]['status'],'after':after[p]['status'],'cli':after[p]['cli']} for p in after if after[p]['status']!='pass']}
assert not delta['lost_passes'] and not delta['new_errors']
model=[]
for p in json.load(open(S/'all1649.json'))[:1595]:
 root=P/('inventory-v1' if p in {r['source'] for r in json.load(open(P/'inventory-v1.json'))['results']} else 'focused-v1')
 old=(root/p).with_suffix('.ast.json');new=(S/'final1649'/p).with_suffix('.ast.json');assert old.exists() and new.exists();assert old.read_bytes()==new.read_bytes(),p
 model.append(p)
delta['unchanged_debug_ast_captures']=len(model)
delta['unchanged_successful_ast_captures']=sum(after[p]['ast']['exit']==0 for p in model)
# The prior20 matrix captured YAML, not debug AST. Compare those structured bytes.
yaml_unchanged=[]
for path in paths:
 src=Path(path);out=S/'models'/(src.stem+'.old20-final.yaml');p=subprocess.run([str(S/'final-rustuml'),'--yaml',str(src)],capture_output=True);out.write_bytes(p.stdout);out.with_suffix('.stderr').write_bytes(p.stderr)
 old=P/'fresh-pairs'/src.with_suffix('.after.yaml').name;assert p.returncode==0 and old.read_bytes()==p.stdout,path;yaml_unchanged.append(path)
delta['unchanged_prior20_yaml']=yaml_unchanged
# Retain the negative CLI statuses, not only the comparator's invalid marker.
delta['invalid_reference_cli']=[{'source':p,'before_cli':before[p].get('cli'),'after_cli':after[p]['cli']} for p in cohorts['checker34'] if after[p]['status']=='invalid_reference']
(S/'deltas-and-residuals.json').write_text(json.dumps(delta,indent=2)+'\n')
for name in ['no_oracle_failure_names.txt','golden_failure_names.txt','golden_ulp_candidates.txt']:
 shutil.copy2(Path('test-diagrams')/name,S/name)
gates={}
for label,name in [('no_oracle','no_oracle_failure_names.txt'),('assisted','golden_failure_names.txt')]:
 b=set((P/name).read_text().splitlines());c=set((S/name).read_text().splitlines());gates[label]={'before_failures':len(b),'after_failures':len(c),'new_failures':sorted(c-b),'repaired':sorted(b-c)};assert b==c
def lint(path):
 return [re.sub(r':\d+:\d+',':LINE:COL',l.strip()) for l in path.read_text().splitlines() if l.startswith('error:') or '--> crates/' in l]
gates['clippy']={'unchanged_diagnostic_sites':lint(P/'corrected-clippy.log')==lint(S/'corrected-clippy.log'),'before':lint(P/'corrected-clippy.log'),'after':lint(S/'corrected-clippy.log')}
(S/'gate-delta.json').write_text(json.dumps(gates,indent=2)+'\n')
print(report['all1649'],report['fresh26']);print('repaired',len(delta['repaired']),'lost',len(delta['lost_passes']),'AST',len(model));print(report['overlaps'])
