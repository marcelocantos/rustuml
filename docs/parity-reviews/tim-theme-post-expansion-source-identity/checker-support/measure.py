# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path('/tmp/rustuml-tim-checker')
phase = sys.argv[1]
deps = ROOT/'target/release/deps'
probe = ROOT/f'{phase}-probe'
command = ['rustc', '--edition=2024', str(ROOT/'probe.rs'), '-L', f'dependency={deps}']
for crate in ['rustuml_parser', 'rustuml_oracle']:
    library = max(deps.glob(f'lib{crate}-*.rlib'), key=lambda p:p.stat().st_mtime_ns)
    command += ['--extern', f'{crate}={library}']
command += ['-o', str(probe)]
subprocess.run(command, check=True)
binary = ROOT/f'{phase}-rustuml'
shutil.copy2(ROOT/'target/release/rustuml', binary)
env = dict(os.environ, PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib', RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')

def measure(path, directory, golden, expand):
    directory.mkdir(parents=True, exist_ok=True)
    svg = directory/f'{phase}.svg'
    result = {'source':str(path)}
    with svg.open('wb') as out:
        p = subprocess.run([str(binary), '-pipe'], input=path.read_bytes(), stdout=out, stderr=subprocess.PIPE, env=env, timeout=90)
    result['cli_exit'] = p.returncode
    result['stderr'] = p.stderr.decode(errors='replace')
    if p.returncode == 0:
        p = subprocess.run([str(probe), 'compare', str(golden), str(svg)], capture_output=True, timeout=30)
        result['comparison_exit'] = p.returncode
        result['comparison'] = (p.stdout+p.stderr).decode(errors='replace')
    result['result'] = 'pass' if result.get('comparison_exit') == 0 else 'fail'
    if expand:
        for mode in ['expand','lines']:
            p = subprocess.run([str(probe), mode, str(path)], capture_output=True, env=env, timeout=30)
            (directory/f'{phase}.{mode}').write_bytes(p.stdout)
            result[mode+'_exit'] = p.returncode
        expected = directory/'java.expanded'
        if expected.exists():
            result['expansion_equal'] = (directory/f'{phase}.expand').read_bytes() == expected.read_bytes()
    return result

if sys.argv[2] == 'fresh':
    data = json.loads((ROOT/'cases.json').read_text())
    todo = [(ROOT/'cases'/name/'source.puml', ROOT/'cases'/name, ROOT/'cases'/name/'java.svg', True) for name in data]
else:
    repo = Path.cwd()
    todo = [(p, ROOT/'isolated'/p.relative_to(repo/'test-diagrams/perturbations').with_suffix(''), p.with_suffix('.svg'), False) for p in sorted((repo/'test-diagrams/perturbations').rglob('*.puml'))]
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
    results = list(pool.map(lambda args: measure(*args), todo))
(ROOT/f'{phase}-{sys.argv[2]}.json').write_text(json.dumps(results, indent=2)+'\n')
print(json.dumps({'phase':phase,'suite':sys.argv[2],'total':len(results),'pass':sum(r['result']=='pass' for r in results),'crash_or_error':sum(r['cli_exit']!=0 for r in results),'invalid_reference':sum(r.get('comparison_exit')==2 for r in results),'expansion_equal':sum(r.get('expansion_equal',False) for r in results)}, indent=2))
