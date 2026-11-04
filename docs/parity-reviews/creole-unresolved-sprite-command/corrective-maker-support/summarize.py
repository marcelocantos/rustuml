# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,subprocess,xml.etree.ElementTree as ET
from pathlib import Path
S=Path(__file__).parent;R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-a295df45')
sha=lambda b:hashlib.sha256(b).hexdigest()
summary={}
for cohort in ['retained','maker18','sprites50','checker38','fresh15']:
 a=S/(cohort+'-after.json');b=S/(cohort+'-before.json')
 if not a.exists() or not b.exists():continue
 before=json.loads(b.read_bytes());after=json.loads(a.read_bytes());assert len(before)==len(after)
 pairs=list(zip(before,after));assert all(b['id']==a['id'] for b,a in pairs)
 v=dict(entries=len(pairs),valid=sum(x['reference_valid'] for x in before),before=sum(x['svg_match'] for x in before),after=sum(x['svg_match'] for x in after),gains=[b['id'] for b,a in pairs if not b['svg_match'] and a['svg_match']],losses=[b['id'] for b,a in pairs if b['svg_match'] and not a['svg_match']],failures=[a['id'] for a in after if a['reference_valid'] and not a['svg_match']],nonpositive=[x['id'] for x in after if not x['reference_valid']],before_panics=[b['id'] for b in before if b['panic']],after_panics=[a['id'] for a in after if a['panic']],changed_svg=[b['id'] for b,a in pairs if b['svg']['stdout_sha256']!=a['svg']['stdout_sha256']],changed_yaml=[b['id'] for b,a in pairs if b['yaml']['stdout_sha256']!=a['yaml']['stdout_sha256']],changed_models=[b['id'] for b,a in pairs if b['model']!=a['model']],exact_source_available=sum('exact_source' in a and a['reference_valid'] for a in after),exact_source_equal=sum(a.get('exact_source',False) and a['reference_valid'] for a in after))
 summary[cohort]=v
 print(cohort,{k:v[k] for k in ['entries','valid','before','after','gains','losses','changed_yaml']},flush=True)
(S/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
# Mechanical preservation audit; not a checker verdict.
historical=json.loads((R/'docs/parity-reviews/creole-unresolved-sprite-command/maker-support/historical-files.json').read_bytes())
assert len(historical)==9731
for row in historical:assert sha((R/row['path']).read_bytes())==row['sha256'],row['path']
import_paths=subprocess.check_output(['git','diff-tree','--no-commit-id','--name-only','-r','83469a4832cf20b8e2e57514841e5c46adf63a6e'],text=True,cwd=R).splitlines()
for p in import_paths:assert (R/p).read_bytes()==subprocess.check_output(['git','show','83469a4832cf20b8e2e57514841e5c46adf63a6e:'+p],cwd=R),p
(S/'historical-integrity.json').write_text(json.dumps(dict(old_files_verified=len(historical),imported_files_verified=len(import_paths),imported_rejection='83469a4832cf20b8e2e57514841e5c46adf63a6e',paths=import_paths),indent=2)+'\n')
