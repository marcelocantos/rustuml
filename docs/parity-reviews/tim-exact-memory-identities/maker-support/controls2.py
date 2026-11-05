# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,os,subprocess,hashlib
from pathlib import Path
R=Path.cwd();S=Path('/Users/marcelo/Library/Caches/rustuml-tim7-maker');P=R/'docs/parity-reviews/tim-builtin-value-boundaries/checker5-support';rows=json.loads((P/'controls.json').read_bytes());out=[]
env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',RAYON_NUM_THREADS='4',TZ='GMT+11:00',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
collateral={r['source']:r for r in json.loads((S/'final2-collateral.json').read_bytes())}
for row in rows:
 p=R/row['source'];d=S/'controls2'/p.stem;d.mkdir(parents=True,exist_ok=True)
 if 'argv' in row:
  cmd=[S/'final2-rustuml',p];q=subprocess.run(cmd,capture_output=True,env=env,cwd=R,timeout=90);(d/'rust.svg').write_bytes(q.stdout);(d/'stderr').write_bytes(q.stderr)
  cmp=[S/'probe','compare',p.with_suffix('.svg'),d/'rust.svg'];c=subprocess.run(cmp,capture_output=True,timeout=90);(d/'comparison.stdout').write_bytes(c.stdout);(d/'comparison.stderr').write_bytes(c.stderr)
  out.append(dict(source=str(p),argv=list(map(str,cmd)),cwd=str(R),exit=q.returncode,comparison_argv=list(map(str,cmp)),comparison_exit=c.returncode,svg_match=q.returncode==c.returncode==0))
 else:
  r=collateral[row['source']];out.append(dict(source=str(p),svg_match=r['current']['svg_match'],source_equal=r['current'].get('source_equal'),provenance='final2-collateral.json; no redundant CLI invocation'))
(S/'controls2.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out,indent=2))
