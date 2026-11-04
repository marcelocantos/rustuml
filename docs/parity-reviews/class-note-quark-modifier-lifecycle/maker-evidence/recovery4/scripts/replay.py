# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from collections import Counter

ROOT = Path.cwd()
S = Path('/Users/marcelo/Library/Caches/rustuml-class-cff4')
ENV = dict(os.environ, PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib', RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()
def run(cmd):
    try:
        p = subprocess.run(cmd, cwd=ROOT, env=ENV, capture_output=True, timeout=60)
        return p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired as e:
        return 124, e.stdout or b'', (e.stderr or b'') + b'CHECKER TIMEOUT'
name, binary, manifest, revision = sys.argv[1:]
dest = S / name
dest.mkdir(exist_ok=True)
paths = json.loads(Path(manifest).read_text())
start = time.time()
def one(path):
    src = ROOT / path
    ref = src.with_suffix('.svg')
    out = dest / path
    out.parent.mkdir(parents=True, exist_ok=True)
    commands = [[binary, '-tsvg', str(src)], [binary, '--ast', str(src)], [str(S/'check_svg'), str(ref), str(out.with_suffix('.rust.svg'))]]
    vals = []
    for cmd, suffix in zip(commands, ['.rust.svg', '.ast.json', '.diff.txt']):
        code, stdout, stderr = run(cmd)
        out.with_suffix(suffix).write_bytes(stdout)
        out.with_suffix(suffix+'.stderr').write_bytes(stderr)
        vals.append(dict(exit=code, stderr=stderr.decode(errors='replace')))
    cli, ast, cmp = vals
    status = 'invalid_reference' if cmp['exit']==2 else 'process_error' if cli['exit'] else 'pass' if cmp['exit']==0 else 'mismatch' if cmp['exit']==1 else 'comparison_error'
    model = None
    try:
        model = json.loads(out.with_suffix('.ast.json').read_text())
    except (ValueError, OSError):
        pass
    return dict(source=path, source_sha256=sha(src), java_sha256=sha(ref), status=status, cli=cli, ast=ast, comparison=cmp, commands=commands, model=model)
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
    results = list(pool.map(one, paths))
report = dict(revision=revision, binary=binary, binary_sha256=sha(Path(binary)), cwd=str(ROOT), environment={k:ENV[k] for k in ['PLANTUML_STDLIB','RUSTUML_DEBUG']}, seconds=time.time()-start, counts=dict(Counter(r['status'] for r in results)), results=results)
(S/(name+'.json')).write_text(json.dumps(report,indent=2)+'\n')
print(name, report['counts'], flush=True)
