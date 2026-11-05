# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,pathlib,xml.etree.ElementTree as E,collections,hashlib
S=pathlib.Path(__file__).parent;R=pathlib.Path('/Users/marcelo/.t3/worktrees/rustuml/t3-ae47bffd');sha=lambda b:hashlib.sha256(b).hexdigest()
def dump(n,v):(S/n).write_text(json.dumps(v,indent=2)+'\n')
def observe(p):
 root=E.fromstring(p.read_bytes());return dict(canvas=root.attrib,text=[dict(text=e.text,attributes=e.attrib) for e in root.iter() if e.tag.endswith('}text')],tags=dict(collections.Counter(e.tag.split('}')[-1] for e in root.iter())),filters=[dict(e.attrib) for e in root.iter() if e.tag.endswith('}feFlood')])
allrows=json.loads((S/'all-outcomes.json').read_bytes());obs=[]
for r in allrows:
 if r['cohort'] not in ['fallback16','checker36','checker32']:continue
 m=r['membership'];obs.append(dict(id=r['id'],before_pass=r['before']['svg_match'],after_pass=r['after']['svg_match'],reference=observe(pathlib.Path(m['golden_capture'])),before=observe(S/'replay/before'/r['id']/'svg.stdout') if not r['before']['malformed_xml'] else None,after=observe(S/'replay/after'/r['id']/'svg.stdout') if not r['after']['malformed_xml'] else None))
dump('component-observations.json',obs)
# Machine-check the bounded claims that make the consumer distinction concrete.
for name in ['image_sequence_base_bold','image_activity_base_bold','image_state_base_bold','image_class_base_bold','image_title_default_bold','image_sequence_base_italic']:
 r=next(x for x in obs if x['id']=='checker32/'+name);jt=r['reference']['text'];rt=r['after']['text'];assert len(jt)==len(rt)
 diffs=[]
 for j,t in zip(jt,rt):
  assert j['text']==t['text'];keys=set(j['attributes'])|set(t['attributes']);delta={k:[j['attributes'].get(k),t['attributes'].get(k)] for k in keys if j['attributes'].get(k)!=t['attributes'].get(k)}
  if delta:
   assert j['text']=='(Cannot\xa0decode)'
   expected='font-style' if name.endswith('italic') else 'font-weight';assert delta=={expected:[None,'italic' if expected=='font-style' else '700']},delta
   diffs.append(delta)
 assert len(diffs)==(1 if name in ['image_title_default_bold','image_sequence_base_italic'] else 2)
# Fonts remain plain in both WBS variants: test intent was not effective base-bold coverage.
for name in ['image_wbs_plain_base','image_wbs_base_bold']:
 r=next(x for x in obs if x['id']=='checker32/'+name)
 assert r['after_pass'];assert all('font-weight' not in t['attributes'] for phase in ['reference','after'] for t in r[phase]['text'])
for n in ['font_invalid_after_valid','font_invalid_member','font_hex_grammar']:
 d=S/'fresh-reference'/n;assert b'NoSuchColorRuntimeException' in (d/'java.svg.stderr').read_bytes();assert b'NoSuchColorRuntimeException' in (d/'java.svg.stdout').read_bytes()
# Stable claimed residuals: compare text runs directly; full comparator still fails.
res=[]
for name in ['color_valid_neighbors','color_registry_boundary','back_invalid_gradient']:
 r=next(x for x in obs if x['id']=='fallback16/'+name);j=r['reference']['text'];a=r['after']['text'];assert len(j)==len(a)
 deltas=[]
 for i,(x,y) in enumerate(zip(j,a)):
  assert x['text']==y['text'];attrs={k:[x['attributes'].get(k),y['attributes'].get(k)] for k in set(x['attributes'])|set(y['attributes']) if x['attributes'].get(k)!=y['attributes'].get(k)}
  if attrs:deltas.append(dict(index=i,text=x['text'],attributes=attrs))
 res.append(dict(case=name,text_deltas=deltas,java_tags=r['reference']['tags'],rust_tags=r['after']['tags'],whole_pass=r['after_pass']))
 assert all(set(x['attributes'])<={'fill' if name!='back_invalid_gradient' else 'filter'} for x in deltas)
dump('residual-cause-check.json',res)
summary=json.loads((S/'summary.json').read_bytes());orig=allrows[:2050];fresh=allrows[2050:]
def loss(rows):return dict(entries=len(rows),valid=sum(r['after']['reference_valid'] for r in rows),before=sum(r['before']['svg_match'] for r in rows),after=sum(r['after']['svg_match'] for r in rows),lost_passes=[r['id'] for r in rows if r['before']['svg_match'] and not r['after']['svg_match']],new_process_errors=[dict(id=r['id'],mode=m) for r in rows for m in ['svg','yaml'] if r['before'][m]['exit']==0 and r['after'][m]['exit']!=0],new_panics=[r['id'] for r in rows if not r['before']['panic'] and r['after']['panic']],retained_svg_panics=sum(r['before']['panic'] and r['after']['panic'] for r in rows),malformed={p:[r['id'] for r in rows if r[p]['malformed_xml']] for p in ['before','after']},new_malformed_xml=[r['id'] for r in rows if not r['before']['malformed_xml'] and r['after']['malformed_xml']],full_yaml_byte_differences=[r['id'] for r in rows if r['before']['yaml']['stdout_sha256']!=r['after']['yaml']['stdout_sha256']])
dump('loss-error-summary.json',dict(final2050=loss(orig),checker32=loss(fresh)))
# Record exact main diagnostic name bytes independently of total counts.
names={}
for mode in ['no_oracle','golden']:
 p=S/'src-after/test-diagrams'/(mode+'_failure_names.txt');b=p.read_bytes();expected=(S/'maker/baseline'/('final-'+mode+'_failure_names.txt')).read_bytes();assert b==expected;names[mode]=dict(names=len(b.splitlines()),sha256=sha(b),byte_identical=True)
 (S/(mode+'_failure_names.txt')).write_bytes(b)
dump('baseline-failure-name-check.json',names)
units=[];log=(S/'logs/library.log').read_text()
for match in __import__('re').finditer(r'Running unittests.*?/deps/(rustuml_[a-z]+)-[^\n]+\n(.*?)(?=\n     Running unittests|\Z)',log,__import__('re').S):
 result=__import__('re').search(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed;',match[2]);units.append(dict(crate=match[1],passed=int(result[1]),failed=int(result[2])))
dump('gate-summary.json',dict(units=units,cli=15,focused=94,product_library=dict(passed=11179,eligible=11251),assisted_library=dict(passed=10497,eligible=11251,total=12550,skipped=1299),names=names,standing=json.loads((S/'standing-summary.json').read_bytes()),guards=dict(passed=4,total=6,failures=['decimal_literal_ratchet','parity_review_records_are_complete']),clippy='Unchanged crates/rustuml-parser/src/parse/component.rs:72 dead code; exit101',workspace_format='Unchanged parser/render class.rs; exit1',owned_creole_format='pass',gate_executions=json.loads((S/'gates.json').read_bytes()),registration='1912 existing pairs tested once; 32 new pairs retained for standing discovery, measured separately through actual CLI and unchanged comparator. New entries do not count as repairs of existing failures.'))
print('bounded observations verified; no whole-output waiver')
