# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,pathlib,hashlib,xml.etree.ElementTree as ET,collections,re
S=pathlib.Path(__file__).parent;R=pathlib.Path('/Users/marcelo/.t3/worktrees/rustuml/t3-ae47bffd');sha=lambda b:hashlib.sha256(b).hexdigest()
cohorts=['retained','maker18','sprites50','checker38','fresh15','fresh26','new15','delimiters4','focused2','checker30','supplement8','fresh12','checker28','fresh16','golden-regressions','checker36','fallback16','checker32'];summary={};allrows=[]
for cohort in cohorts:
 before=json.loads((S/(cohort+'-before.json')).read_bytes());after=json.loads((S/(cohort+'-after.json')).read_bytes());members=json.loads((S/(cohort+'-membership.json')).read_bytes());rows=[]
 for m,b,a in zip(members,before,after):
  assert m['id']==b['id']==a['id'];row=dict(cohort=cohort,id=m['id'],membership=m,before=b,after=a);rows.append(row)
  for phase,v in [('before',b),('after',a)]:
   try:ET.fromstring((S/'replay'/phase/m['id']/'svg.stdout').read_bytes());v['malformed_xml']=False
   except ET.ParseError:v['malformed_xml']=v['svg']['exit']==0
 allrows+=rows
 summary[cohort]=dict(entries=len(rows),valid=sum(x['after']['reference_valid'] for x in rows),before=sum(x['before']['svg_match'] for x in rows),after=sum(x['after']['svg_match'] for x in rows),gains=[x['id'] for x in rows if not x['before']['svg_match'] and x['after']['svg_match']],losses=[x['id'] for x in rows if x['before']['svg_match'] and not x['after']['svg_match']],yaml_byte_changes=[x['id'] for x in rows if x['before']['yaml']['stdout_sha256']!=x['after']['yaml']['stdout_sha256']],new_process_errors=[dict(id=x['id'],mode=m) for x in rows for m in ['svg','yaml'] if x['before'][m]['exit']==0 and x['after'][m]['exit']!=0],new_panics=[x['id'] for x in rows if not x['before']['panic'] and x['after']['panic']],new_malformed_xml=[x['id'] for x in rows if not x['before']['malformed_xml'] and x['after']['malformed_xml']],panics={p:[x['id'] for x in rows if x[p]['panic']] for p in ['before','after']},malformed_xml={p:[x['id'] for x in rows if x[p]['malformed_xml']] for p in ['before','after']},process_exits={p:{m:dict(collections.Counter(x[p][m]['exit'] for x in rows)) for m in ['svg','yaml']} for p in ['before','after']})
(S/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');(S/'all-outcomes.json').write_text(json.dumps(allrows,indent=2)+'\n')
# Independent full-file inventory check.
old=json.loads((R/'docs/parity-reviews/creole-unresolved-sprite-command/maker-support/historical-files.json').read_bytes())
for x in old:assert sha((R/x['path']).read_bytes())==x['sha256'],x['path']
assert len(old)==9731
checks=[]
for r in json.loads((R/'docs/parity-reviews/creole-color-resolution-fallback/maker-support/archive-files.json').read_bytes()):
 b=(S/'maker'/r['path']).read_bytes();assert sha(b)==r['sha256'] and len(b)==r['bytes'];checks.append(r['path'])
(S/'archive-and-inventory-check.json').write_text(json.dumps(dict(archive_members=len(checks),all_member_bytes_equal=True,original_inventory_files=len(old),all_original_inventory_bytes_equal=True),indent=2)+'\n')
for n in ['no_oracle','golden']:
 current=S/'src-after/test-diagrams'/(n+'_failure_names.txt');baseline=S/'maker/baseline'/('final-'+n+'_failure_names.txt')
 assert current.read_bytes()==baseline.read_bytes(),n
 print(n,'failure names',len(current.read_text().splitlines()),'byte exact')
for group,names in [('Final2050',cohorts[:-1]),('Prior1992',cohorts[:14]),('fresh32',['checker32'])]:
 print(group,{k:sum(summary[n][k] for n in names) for k in ['entries','valid','before','after']},'yaml deltas',sum(len(summary[n]['yaml_byte_changes']) for n in names),'losses',sum(len(summary[n]['losses']) for n in names))
# Match the standing cohort by pair hashes, preserving multiplicity.
index=collections.defaultdict(list)
for r in allrows:
 m=r['membership'];index[(m['source_sha256'],m['golden_sha256'])].append(r)
failed={x.strip().split(':',1)[0] for x in (S/'logs/standing.log').read_text().splitlines() if x.startswith('  test-diagrams/perturbations/')}
standing=[]
for p in sorted((S/'src-after/test-diagrams/perturbations').rglob('*.puml')):
 rs=index[(sha(p.read_bytes()),sha(p.with_suffix('.svg').read_bytes()))];rel=str(p.relative_to(S/'src-after'));standing.append(dict(source=rel,harness_pass=rel not in failed,matched=[r['id'] for r in rs],before=any(r['before']['svg_match'] for r in rs),after=any(r['after']['svg_match'] for r in rs)))
ss=dict(entries=len(standing),harness_pass=sum(r['harness_pass'] for r in standing),before_cli=sum(r['before'] for r in standing),after_cli=sum(r['after'] for r in standing),unmapped=[r['source'] for r in standing if not r['matched']],discrepancies=[r['source'] for r in standing if r['harness_pass']!=r['after']])
(S/'standing-summary.json').write_text(json.dumps(ss,indent=2)+'\n');(S/'standing-mapping.json').write_text(json.dumps(standing,indent=2)+'\n');print('standing', {k:v for k,v in ss.items() if k!='discrepancies'})
