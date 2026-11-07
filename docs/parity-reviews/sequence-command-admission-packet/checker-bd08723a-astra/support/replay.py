# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Replay every input with exact output captures; unchanged CLI comparator."""
import concurrent.futures, hashlib, json, os, subprocess, sys
from pathlib import Path
C=Path(__file__).parent; R=Path.cwd(); label=sys.argv[1]; cohort=sys.argv[2]
P=R/'docs/parity-reviews/sequence-command-admission-packet/maker-results'
B=C/(label+'-bin');D=C/(cohort+'-'+label+'-full');D.mkdir(exist_ok=False)
env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
sha=lambda b:hashlib.sha256(b).hexdigest()
items=[dict(source=str(p.relative_to(R)),golden=str(p.with_suffix('.svg').relative_to(R))) for p in sorted((R/'test-diagrams/golden').rglob('*.puml'))] if cohort=='golden' else json.loads((C/(cohort+'-pairs.json') if cohort.startswith('fresh-') else P/'all-inputs.json').read_text())
def run(cmd,data=None):
    try:
        p=subprocess.run(cmd,input=data,cwd=R,env=env,capture_output=True,timeout=90)
        return p.returncode,p.stdout,p.stderr
    except subprocess.TimeoutExpired as e:return 124,e.stdout or b'',b'TIMEOUT '+(e.stderr or b'')
def one(item):
    source=R/item['source'];body=source.read_bytes();stem=sha(item['source'].encode())[:24]
    row=dict(item,source_sha256=sha(body),results={})
    for mode,flags in [('svg',['-tsvg','-pipe']),('ast',['--ast','-pipe']),('ascii',['-ttxt','-pipe']),('yaml',['--yaml','-pipe'])]:
        cmd=[str(B/'rustuml'),*flags];code,stdout,stderr=run(cmd,body)
        f=D/(stem+'.'+mode);f.write_bytes(stdout)
        row['results'][mode]=dict(command=cmd,exit=code,stdout_file=f.name,stdout_sha256=sha(stdout),stderr=stderr.decode(errors='replace'))
        if mode=='svg':
            cmd=[str(B/'compare_cli'),str(R/item['golden']),str(f)];code,stdout,stderr=run(cmd)
            row['comparison']=dict(command=cmd,exit=code,stdout=stdout.decode(errors='replace'),stderr=stderr.decode(errors='replace'))
    return row
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
    rows=[]
    for row in pool.map(one,items):
        rows.append(row)
        if len(rows)%500==0:print(label,cohort,len(rows),flush=True)
(D/'results.json').write_text(json.dumps(rows,indent=2)+'\n')
print('completed',label,cohort,len(rows),flush=True)
