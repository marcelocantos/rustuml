# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,json,subprocess,hashlib,os
from pathlib import Path
C=Path(__file__).parent;D=Path('docs/parity-reviews/sequence-command-contract-repair-packet/maker-results');read=lambda p:json.loads(p.read_text());rows=read(C/'signature4.json');before={r['source']:r for r in read(C/'before-all3111-models.json')};after={r['source']:r for r in read(C/'final-all3111-models.json')};out=[]
for r in rows:
 body=Path(r['source']).read_bytes();modes={}
 for mode,flags in [('model',['--ast','-pipe']),('ascii',['-ttxt','-pipe']),('yaml',['--yaml','-pipe'])]:
  command=[str(C/'candidate2-final-bin/rustuml'),*flags];p=subprocess.run(command,input=body,capture_output=True,env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib'),timeout=30);modes[mode]=dict(command=command,exit=p.returncode,stdout=p.stdout.decode(),stderr=p.stderr.decode(),stdout_sha256=hashlib.sha256(p.stdout).hexdigest())
 out.append(dict(source=r['source'],before=before[r['source']],intermediate70bc20cf1=modes,after=after[r['source']]))
(C/'intermediate-signature-models.json').write_text(json.dumps(out,indent=2)+'\n');(D/'signature-model-witnesses.json.gz').write_bytes(gzip.compress(json.dumps(out,indent=2).encode(),mtime=0))
color={folder:read(C/folder/'measurements.json') for folder in ['before-color29','candidate1-color29','final-color29']};(D/'colored-note-main-cli-diagnostic.json.gz').write_bytes(gzip.compress(json.dumps(color,indent=2).encode(),mtime=0));print({k:sum(r['cli_comparison']['exit']==0 for r in v) for k,v in color.items()})
