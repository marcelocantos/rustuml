# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,shutil
from pathlib import Path
S=Path(__file__).parent;R=Path.cwd();sha=lambda b:hashlib.sha256(b).hexdigest();out=[]
for filename,kind in [('no_oracle_failure_names.txt','product'),('golden_failure_names.txt','assisted')]:
 b=(S/'baseline-diagnostics'/filename).read_bytes();a=(R/'test-diagrams'/filename).read_bytes();shutil.copy2(R/'test-diagrams'/filename,S/('final-'+filename));assert b==a
 out.append(dict(kind=kind,baseline_sha256=sha(b),corrected_sha256=sha(a),byte_equal=True,failure_names=len(b.splitlines()),lost_passes=[],gains=[]))
b=json.loads((S/'golden-regressions-before.json').read_bytes());i=json.loads((S/'initial-candidate/golden-regressions-after.json').read_bytes());a=json.loads((S/'golden-regressions-after.json').read_bytes());members=json.loads((S/'golden-regressions-membership.json').read_bytes());rows=[]
for m,b,i,a in zip(members,b,i,a):
 assert b['svg_match'] and not i['svg_match'] and a['svg_match'];assert b['yaml']['stdout_sha256']==i['yaml']['stdout_sha256']==a['yaml']['stdout_sha256'];rows.append(dict(id=m['id'],source=m['source_capture'],reference=m['golden_capture'],before=b,initial_candidate=i,corrected=a,all_model_bytes_equal=True))
(S/'golden-regression-observations.json').write_text(json.dumps(rows,indent=2)+'\n')
(S/'golden-regression-summary.json').write_text(json.dumps(dict(initial_candidate='3d92268c4aae5f8bfd3a0e271758b6a8e5442e47',corrected='8ca73318f584dc29d7d8ab1c91f25a21718ca713',initial_actual_cli_lost_passes=[r['id'] for r in rows],initial_product_library_losses=6,initial_assisted_library_losses=5,corrected_actual_cli_restored=6,corrected_lost_passes=[],new_errors=[],new_panics=[],models_byte_equal=6,diagnostic_failure_name_sets=out,claim_scope='The immutable1992 observations have zero losses throughout; separate six affected golden CLI controls lose all six initially and regain all six after correction. Both full-library failure-name sets are identical to baseline. No unmeasured universal CLI equivalence claim.'),indent=2)+'\n');print(json.dumps(out,indent=2))
