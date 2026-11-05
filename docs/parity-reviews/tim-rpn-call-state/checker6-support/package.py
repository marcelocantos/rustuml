# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,json,shutil,xml.etree.ElementTree as ET
from pathlib import Path
from replay import R,S,load,dump,sha
D=R/'docs/parity-reviews/tim-rpn-call-state/checker6-support';D.mkdir(exist_ok=True)
fresh=load(S/'fresh25-results.json')+load(S/'fresh4-results.json')
for r in fresh:
 p=Path(r['actual_source']);dest=R/'test-diagrams/perturbations'/r['mechanism']/'checker6';dest.mkdir(parents=True,exist_ok=True)
 shutil.copy2(p,dest/p.name);shutil.copy2(r['actual_golden'],dest/(p.stem+'.svg'))
 ed=D/'cases'/p.stem;ed.mkdir(parents=True,exist_ok=True)
 relay=S/('relay25' if r['suite']=='fresh25' else 'relay4')/p.stem
 for f in relay.iterdir():shutil.copy2(f,ed/f.name)
 captures=S/'replay'/r['suite']/r['capture_id']
 for phase in ['before','after']:shutil.copytree(captures/phase,ed/phase,dirs_exist_ok=True)
 r['committed_source']=str((dest/p.name).relative_to(R));r['committed_golden']=str((dest/(p.stem+'.svg')).relative_to(R));r['committed_evidence']=str(ed.relative_to(R))
 try:r['java_svg_text']=[e.text for e in ET.fromstring(Path(r['actual_golden']).read_bytes()).iter() if e.tag.endswith('text')]
 except ET.ParseError:r['java_svg_text']=[]
 dump(ed/'result.json',r)
dump(S/'fresh-results.json',fresh);dump(D/'fresh-results.json',fresh)
for name in ['remote-request.json','new-manifest.json','supplement-request.json','supplement-manifest.json','gates.json','audit.json','java-source-provenance.json','old-evidence-integrity.json','analysis.json','negative-statuses.json','controls.json','setup-failure.json','production.diff','chronology.log']:
 shutil.copy2(S/name,D/name)
for p in S.glob('*.py'):shutil.copy2(p,D/p.name)
for name in ['collateral','prior31','fresh29','maker15','maker17','maker18']:
 shutil.copy2(S/(name+'-summary.json'),D/(name+'-summary.json'))
shutil.copytree(S/'logs',D/'logs',dirs_exist_ok=True)
for name in ['relay25','relay4']:
 dest=D/name;dest.mkdir(exist_ok=True)
 for p in (S/name).iterdir():
  if p.is_file():shutil.copy2(p,dest/p.name)
summary=dict(total=len(fresh),valid=sum(r['reference_valid'] for r in fresh),negative=sum(not r['reference_valid'] for r in fresh),before_matches=sum(r['phases']['before']['svg_match'] for r in fresh),after_matches=sum(r['phases']['after']['svg_match'] for r in fresh),after_source_matches=sum(r['phases']['after'].get('source_equal',False) for r in fresh),negative_rejected=sum(not r['reference_valid'] and r['phases']['after']['cli_exit']!=0 for r in fresh),fresh_valid_failures=[r['committed_source'] for r in fresh if r['reference_valid'] and not r['phases']['after']['svg_match']],fresh_negative_accepted=[r['committed_source'] for r in fresh if not r['reference_valid'] and r['phases']['after']['cli_exit']==0],new_valid_cli_errors=[r['committed_source'] for r in fresh if r['reference_valid'] and r['phases']['before']['cli_exit']==0 and r['phases']['after']['cli_exit']!=0])
dump(D/'fresh-summary.json',summary);print(json.dumps(summary,indent=2))
