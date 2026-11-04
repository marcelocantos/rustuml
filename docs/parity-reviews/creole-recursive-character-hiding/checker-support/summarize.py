# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,subprocess,collections
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();sha=lambda b:hashlib.sha256(b).hexdigest()
summary={}
for cohort in ['retained','maker18','sprites50','checker38','fresh15','fresh26','new15','delimiters4','focused2','checker30','supplement8','fresh12','checker28']:
 if not (S/(cohort+'-after.json')).exists():continue
 before=json.loads((S/(cohort+'-before.json')).read_bytes());after=json.loads((S/(cohort+'-after.json')).read_bytes());members=json.loads((S/(cohort+'-membership.json')).read_bytes());pairs=list(zip(before,after));assert all(b['id']==a['id'] for b,a in pairs)
 v=dict(entries=len(pairs),distinct_pairs=len({(sha(Path(x['source_capture']).read_bytes()),sha(Path(x['golden_capture']).read_bytes())) for x in members}),valid=sum(x['reference_valid'] for x in before),before=sum(x['svg_match'] for x in before),after=sum(x['svg_match'] for x in after),gains=[b['id'] for b,a in pairs if not b['svg_match'] and a['svg_match']],losses=[b['id'] for b,a in pairs if b['svg_match'] and not a['svg_match']],failures=[a['id'] for a in after if a['reference_valid'] and not a['svg_match']],nonpositive=[x['id'] for x in after if not x['reference_valid']],before_panics=[b['id'] for b in before if b['panic']],after_panics=[a['id'] for a in after if a['panic']],changed_svg=[b['id'] for b,a in pairs if b['svg']['stdout_sha256']!=a['svg']['stdout_sha256']],changed_yaml=[b['id'] for b,a in pairs if b['yaml']['stdout_sha256']!=a['yaml']['stdout_sha256']],changed_projection=[b['id'] for b,a in pairs if b['model']!=a['model']],exact_source_available=sum('exact_source' in a and a['reference_valid'] for a in after),exact_source_equal=sum(a.get('exact_source',False) and a['reference_valid'] for a in after),exits={phase:{mode:dict(collections.Counter(str(x[mode]['exit']) for x in result)) for mode in ['svg','yaml']} for phase,result in [('before',before),('after',after)]})
 summary[cohort]=v;print(cohort,{k:v[k] for k in ['entries','valid','before','after','gains','losses']},flush=True)
(S/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
old=json.loads((R/'docs/parity-reviews/creole-unresolved-sprite-command/maker-support/historical-files.json').read_bytes())
for row in old:assert sha((R/row['path']).read_bytes())==row['sha256'],row['path']
checks=[]
for rev in ['83469a4832cf20b8e2e57514841e5c46adf63a6e','521fe226d428c01128f4379bb7342584772924d7','5ec48be56c4bce12ec646c29ed8e6d4be5e8518b']:
 paths=subprocess.check_output(['git','diff-tree','--no-commit-id','--name-only','-r',rev],text=True).splitlines()
 for p in paths:assert (R/p).read_bytes()==subprocess.check_output(['git','show',rev+':'+p]),p
 checks.append(dict(revision=rev,files=len(paths),paths=paths))
(S/'historical-integrity.json').write_text(json.dumps(dict(old_files=len(old),imports=checks),indent=2)+'\n')
