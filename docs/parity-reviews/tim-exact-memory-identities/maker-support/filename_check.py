# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Preserve the dispatch regression and restored CLI behavior on its frozen source."""
import hashlib,json,os,subprocess
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-tim7-maker');R=Path.cwd();p=R/'test-diagrams/golden/preprocessing/preproc_builtin_filename.puml';d=S/'filename-regression';d.mkdir(exist_ok=True)
env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',TZ='GMT+11:00',RAYON_NUM_THREADS='4');rows=[]
for phase in ['before','final','final2']:
 calls=[];dest=d/phase;dest.mkdir(exist_ok=True)
 for mode,args in [('svg',['-pipe']),('yaml',['--yaml','-pipe'])]:
  cmd=[S/(phase+'-rustuml'),*args];q=subprocess.run(cmd,input=p.read_bytes(),cwd=R,env=env,capture_output=True,timeout=90)
  (dest/('rust.'+mode)).write_bytes(q.stdout);(dest/(mode+'.stderr')).write_bytes(q.stderr)
  calls.append(dict(argv=list(map(str,cmd)),cwd=str(R),exit=q.returncode,stdout_sha256=hashlib.sha256(q.stdout).hexdigest(),stderr_sha256=hashlib.sha256(q.stderr).hexdigest()))
 cmd=[S/'probe','compare',p.with_suffix('.svg'),dest/'rust.svg'];q=subprocess.run(cmd,capture_output=True,timeout=90);(dest/'comparison.stdout').write_bytes(q.stdout);(dest/'comparison.stderr').write_bytes(q.stderr)
 rows.append(dict(phase=phase,calls=calls,comparison_argv=list(map(str,cmd)),comparison_exit=q.returncode,svg_match=calls[0]['exit']==q.returncode==0))
(S/'filename-regression.json').write_text(json.dumps(dict(source=str(p),source_sha256=hashlib.sha256(p.read_bytes()).hexdigest(),reference_sha256=hashlib.sha256(p.with_suffix('.svg').read_bytes()).hexdigest(),env_overrides={k:env[k] for k in ['RUSTUML_DEBUG','TZ','RAYON_NUM_THREADS']},results=rows),indent=2)+'\n');print([(r['phase'],r['svg_match']) for r in rows])
