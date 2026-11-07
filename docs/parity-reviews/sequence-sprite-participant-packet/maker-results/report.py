# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,gzip,hashlib,json,re,xml.etree.ElementTree as ET
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd()
def read(p):return json.loads(Path(p).read_text())
def write(n,v):(C/n).write_text(json.dumps(v,indent=2)+'\n')
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def status(r):
 if r['comparison']['exit']==2:return 'nonpositive'
 if r['results']['svg']['exit']!=0:return 'valid_error'
 return 'match' if r['comparison']['exit']==0 else 'mismatch'
def xml(path):
 try:
  root=ET.parse(path).getroot();tags=collections.Counter(x.tag.rsplit('}',1)[-1] for x in root.iter());return dict(xml_valid=True,dimensions={k:root.attrib.get(k) for k in ['width','height','viewBox']},tags=dict(tags),texts=[''.join(x.itertext()) for x in root.iter() if x.tag.endswith('}text')],participant_heads=sum('participant-head' in x.attrib.get('class','').split() for x in root.iter()))
 except (ET.ParseError,UnicodeError) as e:return dict(xml_valid=False,error=str(e))
before=[];after=[];summaries={};changes=[];contracts=[];semantics=[];model_changes=[];residue=[];family_changes=[];full_remaining=[]
old_contract={r['source']:r for r in read(R/'docs/parity-reviews/sequence-command-admission-packet/maker-results/rejection-contract.json')['results']}
for cohort in ['retained4140','maker30','completion4']:
 b=read(C/('before-'+cohort)/'results.json');a=read(C/('final-'+cohort)/'results.json');assert len(a)==len(b)
 stats=dict(entries=len(a),before=dict(collections.Counter(map(status,b))),after=dict(collections.Counter(map(status,a))),gains=[],lost_matches=[],new_valid_errors=[],lost_rejections=[],recovered_valid_errors=[],new_rejections=[])
 for x,y in zip(b,a):
  assert x['source']==y['source'] and x['source_sha256']==y['source_sha256'];source=x['source'];sx,sy=status(x),status(y);bx=x['results']['svg'];ay=y['results']['svg'];changed_modes=[mode for mode in x['results'] if (x['results'][mode]['exit'],x['results'][mode]['stdout_sha256'])!=(y['results'][mode]['exit'],y['results'][mode]['stdout_sha256'])];diagchange=bx['stderr']!=ay['stderr']
  if sx!='match' and sy=='match':stats['gains'].append(source)
  if sx=='match' and sy!='match':stats['lost_matches'].append(source)
  if sx not in ['nonpositive','valid_error'] and sy=='valid_error':stats['new_valid_errors'].append(source)
  if sx=='valid_error' and sy!='valid_error':stats['recovered_valid_errors'].append(source)
  if sy=='nonpositive':
   prior=old_contract.get(source,{});kind=prior.get('reference_kind')
   if not kind:kind='java_export_error' if y.get('java_exit')==0 else 'java_invalid'
   c=dict(source=source,source_sha256=y['source_sha256'],kind=kind,before_exit=bx['exit'],after_exit=ay['exit'],before_rejected=bx['exit']!=0,after_rejected=ay['exit']!=0,before_diagnostic=bx['stderr'],after_diagnostic=ay['stderr']);contracts.append(c)
   if c['before_rejected'] and not c['after_rejected']:stats['lost_rejections'].append(source)
   if not c['before_rejected'] and c['after_rejected']:stats['new_rejections'].append(source)
  if changed_modes or diagchange:changes.append(dict(source=source,before_status=sx,after_status=sy,changed_modes=changed_modes,diagnostic_changed=diagchange,before=x['results'],after=y['results']))
  if any(m!='svg' for m in changed_modes):model_changes.append(dict(source=source,modes=[m for m in changed_modes if m!='svg']))
  for side,row in [('before',x),('after',y)]:
   diag=row['results']['svg']['stderr'];classes=re.findall(r'warning: sequence command (\S+)',diag)
   if 'signed order is recognized but unsupported' in diag:classes.append('CommandParticipant: signed-order residue')
   if 'SVG sprite declaration' in diag:classes.append('CommandSpriteSvg/CommandSpriteSvgMultiline: materialization residue')
   if 'single-line sprite declaration' in diag:classes.append('single-line sprite command: execution residue')
   if classes:residue.append(dict(source=source,side=side,classes=classes,stderr=diag))
  bdir=C/('before-'+cohort);adir=C/('final-'+cohort)
  bf=(bdir/x['results']['ast']['stdout_file']).read_text().partition('(')[0];af=(adir/y['results']['ast']['stdout_file']).read_text().partition('(')[0]
  if bf!=af:family_changes.append(dict(source=source,before_family=bf or 'error',after_family=af or 'error',before_status=sx,after_status=sy))
  if changed_modes or cohort!='retained4140':
   semantic=dict(source=source,reference=xml(R/y['golden']),before=xml(bdir/bx['stdout_file']),after=xml(adir/ay['stdout_file']))
   for side,folder,row in [('before',bdir,x),('after',adir,y)]:
    ast=(folder/row['results']['ast']['stdout_file']).read_text();semantic[side]['model_participant_count']=len(re.findall(r'^\s+Participant \{$',ast,re.M));semantic[side]['model_message_count']=len(re.findall(r'^\s+Message\($',ast,re.M));semantic[side]['model_create_events']=re.findall(r'^\s+Create\(\s*"([^"]*)"',ast,re.M);semantic[side]['ast_sha256']=row['results']['ast']['stdout_sha256'];semantic[side]['yaml_sha256']=row['results']['yaml']['stdout_sha256'];semantic[side]['ascii_sha256']=row['results']['ascii']['stdout_sha256']
   semantics.append(semantic)
  if sy!='match':full_remaining.append(dict(source=source,source_sha256=y['source_sha256'],status=sy,cli_exit=ay['exit'],comparison=y['comparison'],diagnostic=ay['stderr']))
 before+=b;after+=a;summaries[cohort]=stats
write('before.json',before);write('after.json',after);write('cohort-summary.json',summaries);write('changes.json',changes);write('rejection-contract.json',dict(entries=len(contracts),groups={kind:dict(entries=len(rows),before_rejected=sum(r['before_rejected'] for r in rows),after_rejected=sum(r['after_rejected'] for r in rows)) for kind in sorted(set(r['kind'] for r in contracts)) if (rows:=[r for r in contracts if r['kind']==kind])},results=contracts));write('semantic-inventory.json',semantics);write('model-changes.json',model_changes);write('unimplemented-command-residue.json',residue);write('family-changes.json',family_changes);write('remaining-failures.json',full_remaining)
source_groups=collections.defaultdict(list)
for r in after:source_groups[r['source_sha256']].append(r['source'])
write('denominators.json',dict(entries=len(after),paths=len(set(r['source'] for r in after)),distinct_source_hashes=len(source_groups),duplicates=[dict(sha256=k,sources=v) for k,v in source_groups.items() if len(v)>1],cohorts={'retained_previous':3569,'independent_checker_bd087':571,'new_maker':34},checker_standing_aliases_not_added=6,rule='Memberships retained; duplicates listed, never removed. Main goldens are a separate denominator.'))
gb=read(C/'before-golden-cli/results.json');ga=read(C/'final-golden-cli/results.json');gc=read(C/'before-golden-cli/comparisons.json');changes=[]
for x,y,c in zip(gb,ga,gc):
 assert x['source']==y['source']==c['source']
 if (x['exit'],x['stdout_sha256'])!=(y['exit'],y['stdout_sha256']):changes.append(dict(source=x['source'],before=x,after=y,before_comparison=c))
write('golden-byte-changes.json',changes)
# Exact byte identity implies unchanged comparator outcomes; avoid redundant
# comparator processes for equal bytes, but compare every changed output.
import subprocess
comparisons=[]
for x,y,c in zip(gb,ga,gc):
 if x['stdout_sha256']==y['stdout_sha256']:result=c['comparison'];basis='byte-identical actual CLI output'
 else:
  p=subprocess.run([str(C/'before-bin/compare_cli'),str(Path(y['source']).with_suffix('.svg')),str(C/'final-golden-cli'/y['stdout_file'])],capture_output=True);result=dict(exit=p.returncode,stdout=p.stdout.decode(errors='replace'),stderr=p.stderr.decode(errors='replace'));basis='unchanged comparator process'
 comparisons.append(dict(source=y['source'],comparison=result,basis=basis))
write('golden-final-comparisons.json',comparisons)
golden_models={}
for side in ['before','final']:
 p=C/(side+'-goldens12550-models.json')
 if p.exists():golden_models[side]=read(p)
if len(golden_models)==2:
 changes=[]
 for x,y in zip(golden_models['before'],golden_models['final']):
  assert x['source']==y['source'];modes=[m for m in x['results'] if (x['results'][m]['exit'],x['results'][m]['stdout_sha256'])!=(y['results'][m]['exit'],y['results'][m]['stdout_sha256'])]
  if modes:changes.append(dict(source=x['source'],modes=modes,before=x,after=y))
 write('golden-model-changes.json',changes)
summary=dict(entries=len(after),before=dict(collections.Counter(map(status,before))),after=dict(collections.Counter(map(status,after))),gains=sum(len(s['gains']) for s in summaries.values()),lost_matches=sum(len(s['lost_matches']) for s in summaries.values()),lost_rejections=sum(len(s['lost_rejections']) for s in summaries.values()),new_valid_errors=sum(len(s['new_valid_errors']) for s in summaries.values()),new_invalid_rejections=sum(len(s['new_rejections']) for s in summaries.values()),timeouts=sum(r['results'][m]['timeout'] for r in after for m in r['results']),cli_panics=sum('panicked at' in r['results']['svg']['stderr'] for r in after),goldens=dict(entries=len(ga),comparison_counts=dict(collections.Counter(r['comparison']['exit'] for r in comparisons)),changed_svg_outputs=len(read(C/'golden-byte-changes.json')),new_valid_errors=[y['source'] for x,y in zip(gb,ga) if x['exit']==0 and y['exit']!=0],models_complete=len(golden_models)==2))
write('summary.json',summary);print(json.dumps(summary,indent=2))
