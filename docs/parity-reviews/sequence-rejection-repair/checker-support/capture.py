# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Capture fresh raw Java references through the remote authorized wrapper only."""
import concurrent.futures
import hashlib
import json
from pathlib import Path
import re
import shlex
import subprocess
import sys
ROOT=Path(__file__).resolve().parents[4]
REPORT=Path(__file__).resolve().parent
CACHE=Path('/Users/marcelo/Library/Caches/rustuml-sequence-review-d382aba6')
endpoint=sys.argv[1]
assert re.fullmatch(r'[a-zA-Z0-9@.:-]+',endpoint)
ssh=['ssh','-T','-o','BatchMode=yes','-o','ConnectTimeout=10',endpoint]
allocation=subprocess.run(ssh+['/Users/marcelo/rustuml-reference/bin/new-worker'],capture_output=True,timeout=30)
(CACHE/'remote-allocation.stdout').write_bytes(allocation.stdout)
(CACHE/'remote-allocation.stderr').write_bytes(allocation.stderr)
allocation.check_returncode()
worker=allocation.stdout.decode().strip()
assert re.fullmatch(r'/Users/marcelo/rustuml-reference/scratch/worker\.[a-zA-Z0-9]+',worker),worker
ready=subprocess.run(ssh+['cat /Users/marcelo/rustuml-reference/readiness.json'],capture_output=True,timeout=30)
ready.check_returncode()
(REPORT/'reference-readiness.json').write_bytes(ready.stdout)
command=ssh+[shlex.join(['/Users/marcelo/rustuml-reference/bin/reference','--cwd',worker,'--timezone','GMT+11:00','--','-pipe','-tsvg'])]
rows=json.loads((REPORT/'fresh-manifest.json').read_text())
def sha(data): return hashlib.sha256(data).hexdigest()
def one(row):
 source=ROOT/row['source'];golden=ROOT/row['golden']
 assert not golden.exists(),golden
 result=subprocess.run(command,input=source.read_bytes(),capture_output=True,timeout=120)
 if result.returncode==255 or b'<svg' not in result.stdout:
  failure=dict(row,command=command,exit=result.returncode,stdout=result.stdout.decode(errors='replace'),stderr=result.stderr.decode(errors='replace'))
  (CACHE/(source.stem+'.transport-failure.json')).write_text(json.dumps(failure,indent=2)+'\n')
  raise RuntimeError('Reference transport/render failure: '+str(source))
 golden.write_bytes(result.stdout)
 source.with_suffix('.stderr').write_bytes(result.stderr)
 provenance=dict(command=command,exit=result.returncode,source_sha256=sha(source.read_bytes()),svg_sha256=sha(result.stdout),stderr_sha256=sha(result.stderr),input_frontend='raw CLI stdin, unmodified source bytes',remote_cwd=worker,host='Monkey Man',endpoint=endpoint,timezone='GMT+11:00',java_source_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',java='Homebrew OpenJDK 21.0.5',graphviz='12.2.0',environment_limit='Seven prior calibration pairs establish scoped agreement only; not universal equivalence with origin Temurin21.0.13/Graphviz16.1.',local_java_launched=False)
 source.with_suffix('.provenance.json').write_text(json.dumps(provenance,indent=2)+'\n')
 print(source.name,result.returncode,len(result.stdout),flush=True)
 return dict(row,**provenance)
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
 captured=list(pool.map(one,rows))
(REPORT/'reference-manifest.json').write_text(json.dumps(captured,indent=2)+'\n')
