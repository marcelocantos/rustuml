# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from replay import R,S,load,dump,run,sha
import os
rows=load(R/'docs/parity-reviews/tim-builtin-value-boundaries/checker5-support/controls.json');out=[]
env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',RAYON_NUM_THREADS='4',TZ='GMT+11:00',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
collateral={r['source']:r for r in load(S/'collateral-results.json')}
for row in rows:
 p=R/row['source'];r=dict(source=str(p),phases={})
 if 'argv' in row:
  for phase in ['before','after']:
   d=S/'controls'/p.stem/phase;d.mkdir(parents=True,exist_ok=True);cmd=[S/(phase+'-rustuml'),p];q=run(cmd,R,env);(d/'rust.svg').write_bytes(q.stdout);(d/'stderr').write_bytes(q.stderr)
   cmp=[S/'probe','compare',p.with_suffix('.svg'),d/'rust.svg'];c=run(cmp,R,env);(d/'comparison.stdout').write_bytes(c.stdout);(d/'comparison.stderr').write_bytes(c.stderr)
   r['phases'][phase]=dict(argv=list(map(str,cmd)),cwd=str(R),env_overrides={k:env[k] for k in ['RUSTUML_DEBUG','RAYON_NUM_THREADS','TZ','PLANTUML_STDLIB']},exit=q.returncode,stdout_sha256=sha(q.stdout),stderr_sha256=sha(q.stderr),comparison_argv=list(map(str,cmp)),comparison_exit=c.returncode,svg_match=q.returncode==c.returncode==0)
 else:
  z=collateral[row['source']];r.update(phases=z['phases'],provenance='independent collateral-results replay, not another invocation')
 out.append(r)
dump(S/'controls.json',out);print([(r['source'],r['phases']['before']['svg_match'],r['phases']['after']['svg_match']) for r in out])
