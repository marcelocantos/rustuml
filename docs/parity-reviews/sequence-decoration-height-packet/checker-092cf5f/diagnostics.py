# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib, json, os, subprocess
from pathlib import Path
cache=Path(__file__).parent; root=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-607d50f7')
env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def run(argv,body=None):
    p=subprocess.run([str(x) for x in argv],cwd=root,env=env,input=body,capture_output=True)
    return dict(command=[str(x) for x in argv],cwd=str(root),exit=p.returncode,stdout=p.stdout.decode(),stderr=p.stderr.decode())
deps=cache/'target-after/release/deps'
argv=['rustc','--edition=2024',cache/'model_retention.rs','-L','dependency='+str(deps),'-o',cache/'model-retention']
for name in ['rustuml_parser','serde_json']:
    libs=list(deps.glob('lib'+name+'-*.rlib')); assert len(libs)==1
    argv+=['--extern',name+'='+str(libs[0])]
build=run(argv); assert build['exit']==0,build
retention=run([cache/'model-retention']); assert retention['exit']==0,retention
(cache/'model-retention.json').write_text(json.dumps(dict(build=build,run=retention,source_sha256=sha(cache/'model_retention.rs')),indent=2)+'\n')
dest=cache/'main-regression'; dest.mkdir(exist_ok=False)
source=root/'test-diagrams/golden/creole/creole_underline_in_sequence.puml'
rows=[]
for revision in ['before','intermediate','after']:
    result=run([cache/('bin-'+revision)/'rustuml','-tsvg','-pipe'],source.read_bytes())
    output=dest/(revision+'.svg'); output.write_text(result.pop('stdout'))
    compare=run([cache/'bin-after/compare_cli',source.with_suffix('.svg'),output])
    rows.append(dict(revision=revision,source=str(source),source_sha256=sha(source),golden_sha256=sha(source.with_suffix('.svg')),result=result,comparison=compare,output_sha256=sha(output)))
(dest/'results.json').write_text(json.dumps(rows,indent=2)+'\n')
ast=cache/'asts'; ast.mkdir(exist_ok=False)
ascii=[]
for case in json.loads((cache/'reference-manifest.json').read_text())['cases']:
    if case['intended_reference']!='valid sequence': continue
    body=Path(case['source']).read_bytes()
    a=run([cache/'bin-after/rustuml','--ast','-pipe'],body)
    (ast/(case['name']+'.json')).write_text(json.dumps(a,indent=2)+'\n')
    results={name:run([cache/('bin-'+name)/'rustuml','-ttxt','-pipe'],body) for name in ['before','after']}
    ascii.append(dict(name=case['name'],same=all(results['before'][k]==results['after'][k] for k in ['exit','stdout','stderr']),results=results))
(cache/'ascii.json').write_text(json.dumps(ascii,indent=2)+'\n')
print('underline:',[(r['revision'],r['comparison']['exit']) for r in rows])
print('ascii:',sum(r['same'] for r in ascii),'/',len(ascii))
print(retention['stdout'])
