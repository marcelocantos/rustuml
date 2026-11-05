# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import importlib.util,json,shutil
from pathlib import Path
spec=importlib.util.spec_from_file_location('c','/tmp/rustuml-tim-check-2fa74908/checker.py');c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
rows=[]
for n,expr in [('sum_wrap','2147483647 + 1'),('difference_wrap','(-2147483647 - 1) - 1')]:
 p=Path('test-diagrams/perturbations/tim-date-pattern-tokens')/('checker2_'+n+'.puml');c.save(p,('@startuml\n!$stamp = %date("yyyy-MM-dd HH:mm:ss",'+expr+')\nclass "$stamp" as Wrapped\n@enduml\n').encode())
 rows.append(c.measure(dict(source=str(p),mechanism='tim-date-pattern-tokens',kind='fresh arithmetic boundary',axes=['Java signed integer wrap','explicit date arithmetic expression']),'candidate',True))
c.dump(c.SUP/'candidate-arithmetic.json',rows)
for r in rows:print(Path(r['source']).stem,r['java_exit'],r['cli_exit'],r['svg_match'],flush=True)
rows=[]
manifest=json.loads((c.SUP/'supplement-manifest.json').read_text())
for r in manifest:
 if r['mechanism']!='tim-evaluated-event-stream':continue
 p=Path(r['source']);out=c.S/'file-reader'/p.stem;out.mkdir(parents=True,exist_ok=True);source=out/p.name;shutil.copy2(p,source)
 j=c.run([c.JAVA,'-Djava.awt.headless=true','-Duser.timezone=GMT+11:00','-jar',c.JAR,'-charset','UTF-8','-tsvg',source]);c.save(out/'java.stderr',j.stderr)
 a=c.run([c.S/'candidate-rustuml',source]);c.save(out/'rust.svg',a.stdout);c.save(out/'rust.stderr',a.stderr)
 java_svg=out/'canopy.svg' if 'raw_named_trailing' in p.stem else source.with_suffix('.svg')
 cmp=c.run([c.S/'probe','compare',java_svg,out/'rust.svg']);c.save(out/'comparison.txt',cmp.stdout+cmp.stderr)
 pipe=c.run([c.S/'probe','compare',p.with_suffix('.svg'),java_svg]);c.save(out/'pipe-file-comparison.txt',pipe.stdout+pipe.stderr)
 rows.append(dict(source=str(p),java_exit=j.returncode,rust_exit=a.returncode,comparison_exit=cmp.returncode,java_pipe_file_comparison_exit=pipe.returncode))
c.dump(c.SUP/'file-reader-results.json',rows)
p=Path('test-diagrams/perturbations/tim-parser-error-propagation/checker2_multi_block_error.puml');r=c.run([c.S/'candidate-rustuml','--block=1','-pipe'],p.read_bytes());c.save(c.S/'multi-block-selected.stdout',r.stdout);c.save(c.S/'multi-block-selected.stderr',r.stderr);c.dump(c.SUP/'multi-block-selected.json',dict(source=str(p),command=[str(c.S/'candidate-rustuml'),'--block=1','-pipe'],exit=r.returncode,stdout_bytes=len(r.stdout),stderr=r.stderr.decode(),note='Default Rust selects block 0; Java -pipe emits all blocks. This selected block experiment exercises the failing block explicitly, not a second preprocessing pass.'))
print('files',rows,'block',r.returncode,len(r.stdout),flush=True)
