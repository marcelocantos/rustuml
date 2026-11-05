# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Execute retained rejection inputs; exit 1 if either a known or introduced false accept remains.

Usage: uv run --no-project python rejection_contract.py REPO_ROOT FRESH_PAIRS BEFORE_BIN AFTER_BIN OUTPUT_JSON
No Java launch, comparator alteration or validity inference from Rust output.
"""
import hashlib,json,os,subprocess,sys
from pathlib import Path
root,manifest,before,after,output=map(Path,sys.argv[1:]);cases=json.loads(manifest.read_text());results=[]
env=dict(os.environ,RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib')
for item in cases:
 if item['observed_valid']:continue
 row=dict(source=item['source'],intended_valid=item['intended_valid'],observed_valid=False,results={});body=(root/item['source']).read_bytes()
 for label,binary in [('before',before),('after',after)]:
  argv=[str(binary),'-tsvg','-pipe'];p=subprocess.run(argv,input=body,capture_output=True,cwd=root,env=env,timeout=30);row['results'][label]=dict(argv=argv,exit=p.returncode,stdout_sha256=hashlib.sha256(p.stdout).hexdigest(),stderr=p.stderr.decode())
 row['introduced_rejection_loss']=row['results']['before']['exit']!=0 and row['results']['after']['exit']==0;results.append(row)
summary=dict(entries=len(results),before_rejections=sum(r['results']['before']['exit']!=0 for r in results),after_rejections=sum(r['results']['after']['exit']!=0 for r in results),introduced_losses=[r['source'] for r in results if r['introduced_rejection_loss']],after_false_accepts=[r['source'] for r in results if r['results']['after']['exit']==0],results=results)
output.write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps({k:v for k,v in summary.items() if k!='results'},indent=2));sys.exit(bool(summary['after_false_accepts']))
