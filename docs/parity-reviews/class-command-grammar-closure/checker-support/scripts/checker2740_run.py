# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Isolated shipped-CLI replay; unchanged rustuml_oracle comparator. Fail closed."""
from pathlib import Path
from collections import Counter, defaultdict
import concurrent.futures
import hashlib
import json
import os
import subprocess
import sys
import time

ROOT = Path.cwd()
OUT = Path(os.environ.get('CHECKER_OUTPUT', '/tmp/rustuml-checker2740'))
BINARY = Path(os.environ.get('CHECKER_BINARY', '/tmp/rustuml-checker2740-target/release/rustuml'))
COMPARATOR = Path('/tmp/rustuml-checker2740/check_svg')
ENV = dict(os.environ, PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib', RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')

def run_one(item):
    cohort, source, relative = item
    destination = OUT / 'outputs' / cohort / relative.with_suffix('')
    destination.mkdir(parents=True, exist_ok=True)
    commands = [[str(BINARY), '-tsvg', str(source)], [str(BINARY), '--ast', str(source)], [str(COMPARATOR), str(source.with_suffix('.svg')), str(destination/'rust.svg')]]
    results = []
    for command, name in zip(commands, ['rust.svg', 'ast.txt', 'strict.txt']):
        try:
            proc = subprocess.run(command, cwd=ROOT, env=ENV, capture_output=True, timeout=60)
            stdout, stderr, code = proc.stdout, proc.stderr, proc.returncode
        except subprocess.TimeoutExpired as error:
            stdout, stderr, code = error.stdout or b'', (error.stderr or b'') + b'\nCHECKER TIMEOUT\n', 124
        (destination/name).write_bytes(stdout)
        (destination/(name+'.stderr')).write_bytes(stderr)
        results.append({'exit': code, 'stdout_sha256': hashlib.sha256(stdout).hexdigest(), 'stderr': stderr.decode(errors='replace')})
    cli, ast, strict = results
    status = 'invalid_reference' if strict['exit'] == 2 else 'process_error' if cli['exit'] else 'pass' if strict['exit'] == 0 else 'mismatch' if strict['exit'] == 1 else 'comparison_error'
    record = {'cohort':cohort, 'source':str(source), 'relative':str(relative), 'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(), 'java_sha256':hashlib.sha256(source.with_suffix('.svg').read_bytes()).hexdigest(), 'status':status, 'commands':commands,'cli':cli,'ast':ast,'comparison':strict,'output':str(destination)}
    (destination/'result.json').write_text(json.dumps(record,indent=2)+'\n')
    return record

def main():
    mode = sys.argv[1]
    if mode == 'current':
        base = ROOT/'test-diagrams/perturbations'
        items = [('current955',p,p.relative_to(base)) for p in sorted(base.rglob('*.puml'))]
        assert len(items)==955,len(items)
    elif mode == 'maker':
        base=ROOT/'docs/parity-reviews/class-command-grammar-closure/maker-evidence/inputs'
        items=[(cohort,p,p.relative_to(base/cohort)) for cohort in ['prior31','fresh'] for p in sorted((base/cohort).glob('*.puml'))]
        assert len(items)==62,len(items)
    elif mode == 'fresh':
        base=OUT/'new-sources'
        items=[('checker_fresh',p,p.relative_to(base)) for p in sorted(base.rglob('*.puml'))]
    else:
        raise ValueError(mode)
    started=time.time()
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        results=list(pool.map(run_one,items))
    counts=defaultdict(Counter)
    for record in results:
        counts[record['cohort']][record['status']]+=1
    report={'mode':mode,'tested_revision':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'cwd':str(ROOT),'environment':{k:ENV[k] for k in ['PLANTUML_STDLIB','RUSTUML_DEBUG']},'binary_sha256':hashlib.sha256(BINARY.read_bytes()).hexdigest(),'comparator_sha256':hashlib.sha256(COMPARATOR.read_bytes()).hexdigest(),'seconds':time.time()-started,'counts':dict(counts),'results':results}
    (OUT/(mode+'-results.json')).write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report['counts'],indent=2),flush=True)
    return 1 if any(r['status']!='pass' for r in results) else 0

if __name__=='__main__':
    sys.exit(main())
