# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,os,subprocess,hashlib
from pathlib import Path
R=Path.cwd();S=Path('/Users/marcelo/Library/Caches/rustuml-tim8-maker');rows=json.loads((R/'docs/parity-reviews/tim-builtin-value-boundaries/checker5-support/controls.json').read_bytes());out=[]
env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',RAYON_NUM_THREADS='4',TZ='GMT+11:00',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib');collateral={r['membership']['source']:r for r in json.loads((S/'iteration2-collateral.json').read_bytes())}
for row in rows:
 p=R/row['source'];d=S/'controls'/p.stem;d.mkdir(parents=True,exist_ok=True)
 if 'argv' in row:
  cmd=[S/'final-rustuml',p];q=subprocess.run(cmd,capture_output=True,env=env,cwd=R,timeout=90);(d/'rust.svg').write_bytes(q.stdout);(d/'stderr').write_bytes(q.stderr)
  cmp=[S/'probe','compare',p.with_suffix('.svg'),d/'rust.svg'];c=subprocess.run(cmp,capture_output=True,timeout=90);(d/'comparison.stdout').write_bytes(c.stdout);(d/'comparison.stderr').write_bytes(c.stderr)
  out.append(dict(source=str(p),argv=list(map(str,cmd)),cwd=str(R),exit=q.returncode,comparison_argv=list(map(str,cmp)),comparison_exit=c.returncode,svg_match=q.returncode==c.returncode==0))
 else:
  r=collateral[row['source']]['phases']['iteration2'];out.append(dict(source=str(p),svg_match=r['svg_match'],source_equal=r.get('exact_source_equal'),provenance='iteration2-collateral.json; identical final binary, no duplicate execution'))
(S/'controls.json').write_text(json.dumps(out,indent=2)+'\n');print('controls',len(out),'passed',sum(r['svg_match'] for r in out))
