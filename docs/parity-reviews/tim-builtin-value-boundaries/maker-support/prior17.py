# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,os,subprocess
from pathlib import Path
import yaml
R=Path.cwd();S=Path('/tmp/rustuml-tim5');P=R/'docs/parity-reviews/tim-expression-value-snapshot/maker-support/relay-completion'
results=[]
for old in json.loads((P/'results.json').read_bytes()):
 p=Path(old['source']);out=S/'prior17'/p.stem;out.mkdir(parents=True,exist_ok=True)
 env=dict(os.environ,TZ='GMT+11:00',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',RAYON_NUM_THREADS='4');calls=[]
 for mode,args in [('svg',[]),('yaml',['--yaml'])]:
  cmd=[S/'iteration1-rustuml',*args,'-pipe'];q=subprocess.run(cmd,input=p.read_bytes(),cwd=p.parent,env=env,capture_output=True,timeout=90)
  (out/('rust.'+mode)).write_bytes(q.stdout);(out/(mode+'.stderr')).write_bytes(q.stderr);calls.append(dict(argv=list(map(str,cmd)),cwd=str(p.parent),exit=q.returncode))
 cmd=[S/'probe','compare',P/p.stem/'java.svg',out/'rust.svg'];q=subprocess.run(cmd,capture_output=True);(out/'comparison.txt').write_bytes(q.stdout+q.stderr)
 source_equal=False
 try:
  node=yaml.compose((out/'rust.yaml').read_bytes())
  for key in ['diagram','meta','source']:node={k.value:v for k,v in node.value}[key]
  (out/'rust.source').write_text(node.value);source_equal=node.value.encode()==(P/p.stem/'java.source').read_bytes()
 except (TypeError,ValueError,KeyError,AttributeError,yaml.YAMLError):pass
 results.append(dict(source=str(p),reference_valid=old['java_valid_reference'],java_exit=old['java_exit'],cli_exit=calls[0]['exit'],svg_match=old['java_valid_reference'] and calls[0]['exit']==0 and q.returncode==0,source_equal=old['java_valid_reference'] and source_equal,calls=calls,comparison_argv=list(map(str,cmd))))
(S/'prior17-results.json').write_text(json.dumps(results,indent=2)+'\n')
print('prior17',len(results),'valid',sum(r['reference_valid'] for r in results),'svg',sum(r['svg_match'] for r in results),'source',sum(r['source_equal'] for r in results),'negative_rejections',sum(not r['reference_valid'] and r['cli_exit']!=0 for r in results))
