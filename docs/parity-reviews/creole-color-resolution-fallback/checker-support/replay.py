# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Independent CLI replay; no production code, baselines or references written."""
import concurrent.futures
import hashlib
import json
import os
import re
import subprocess
import sys
import time
import xml.etree.ElementTree as ET
from pathlib import Path
import yaml

R = Path('/Users/marcelo/Library/Caches/rustuml-creole-fallback-checker-ae47/src-after')
S = Path('/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-maker')
M = Path('/Users/marcelo/Library/Caches/rustuml-tim8-maker')
SUITES = ['collateral','prior31','fresh29','maker15','maker17','maker18','fresh6','newmaker15','supplement2','fresh23']
sha = lambda b: hashlib.sha256(b).hexdigest()
def dump(p, value):
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(value, indent=2) + '\n')
def resolve(path):
    return R / re.sub(r'/Users/marcelo/\.t3/worktrees/rustuml/t3-[^/]+', str(R), str(path))
def freeze():
    rows = []
    for suite in SUITES:
        membership_file = M / 'replay-inputs' / (suite + '-results.json')
        raw = membership_file.read_bytes()
        (S / (suite + '-original-membership.json')).write_bytes(raw)
        for n, original in enumerate(json.loads(raw)):
            m = original['membership']
            rows.append(dict(suite=suite, index=n, membership=m, original_classification=original.get('classification')))
    for suite, folder in [('maker13','relay13-iteration2'),('maker2','relay2-iteration2')]:
        raw = (M / folder / 'results.json').read_bytes()
        (S / (suite + '-original-membership.json')).write_bytes(raw)
        for n, original in enumerate(json.loads(raw)):
            m = dict(original.get('row', {k:v for k,v in original.items() if k not in ['measurements']}))
            p = Path(m['source']); d = M / folder / p.stem
            m.update(cwd=str(p.parent), golden=str(d/'java.svg.stdout'), reference_source=str(d/'java.source.stdout'),
                     source_sha256=sha(p.read_bytes()), golden_sha256=sha((d/'java.svg.stdout').read_bytes()),
                     reference_valid=original['reference_valid'])
            rows.append(dict(suite=suite, index=n, membership=m, original_classification=original.get('reference_category')))
    archived_paths = json.loads((M/'replay-input-paths.json').read_bytes())
    for row in rows:
        m = row['membership']; row['id'] = f"{row['suite']}/{row['index']:04}-{Path(m['source']).stem}"
        d = S / 'inputs' / row['id']; d.mkdir(parents=True, exist_ok=True)
        for field, file in [('source','input.puml'),('golden','java.svg'),('reference_source','java.source')]:
            path = m.get(field, str(Path(m['source']).with_suffix('.svg')) if field == 'golden' else None)
            if path is None: continue
            actual = resolve(path)
            if actual.exists():
                data = actual.read_bytes()
            else:
                entry = archived_paths[str(path)]
                data = (M/'replay-input-bytes'/entry['sha256']).read_bytes()
                assert sha(data) == entry['sha256']
            if field in ['source','golden']: assert sha(data) == m[field+'_sha256'], path
            (d/file).write_bytes(data)
            row[field+'_capture'] = str(d/file)
            row[field+'_sha256'] = sha(data)
        row['cwd'] = str(resolve(m.get('cwd','.')))
        assert Path(row['cwd']).is_dir(), row['cwd']
    dump(S/'membership.json', rows)
    print('frozen',len(rows),'distinct',len({(r['source_sha256'],r['golden_sha256']) for r in rows}),flush=True)
def call(argv, cwd, env, data, d, label):
    start = time.monotonic()
    try:
        q = subprocess.run(list(map(str,argv)), cwd=cwd, env=env, input=data, capture_output=True, timeout=90)
        code, stdout, stderr = q.returncode, q.stdout, q.stderr
    except subprocess.TimeoutExpired as e:
        code, stdout, stderr = 124, e.stdout or b'', e.stderr or b''
    (d/(label+'.stdout')).write_bytes(stdout); (d/(label+'.stderr')).write_bytes(stderr)
    return dict(argv=list(map(str,argv)), cwd=str(cwd), exit=code, seconds=time.monotonic()-start,
                stdout_sha256=sha(stdout), stderr_sha256=sha(stderr))
def model(data):
    node = yaml.compose(data)
    root = {k.value:v for k,v in node.value}; d = {k.value:v for k,v in root['diagram'].value}
    meta = {k.value:v for k,v in d['meta'].value}
    return dict(type=root['type'].value, entities=len(d['entities'].value) if 'entities' in d else None, source=meta['source'].value)
def measure(row, phase):
    m = row['membership']; d = S/'replay'/phase/row['id']; d.mkdir(parents=True,exist_ok=True)
    env_overrides = dict(TZ=m.get('zone','GMT+11:00'),RUSTUML_DEBUG=m.get('debug','date=1774210426000,tz=AEDT+1100'),
                         RAYON_NUM_THREADS='4', PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib') | m.get('env',{})
    env = dict(os.environ, **env_overrides)
    binary = S/(phase+'-rustuml'); data = Path(row['source_capture']).read_bytes()
    svg = call([binary,'-pipe'],row['cwd'],env,data,d,'svg')
    ym = call([binary,'--yaml','-pipe'],row['cwd'],env,data,d,'yaml')
    cmp = call([S/'probe','compare',row['golden_capture'],d/'svg.stdout'],R,env,None,d,'compare')
    gold = Path(row['golden_capture']).read_bytes()
    try:
        root = ET.fromstring(gold); xml = True
        welcome = any('Welcome to PlantUML!' in (n.text or '') for n in root.iter())
        factory = root.get('data-diagram-type')
    except ET.ParseError:
        xml,welcome,factory = False,False,None
    valid = xml and cmp['exit'] != 2 and not welcome
    result = dict(id=row['id'], phase=phase, env_overrides=env_overrides, svg=svg, yaml=ym, compare=cmp,
                  reference_xml_valid=xml, reference_welcome=welcome, reference_factory=factory,
                  reference_valid=valid, membership_valid_agrees=valid==m['reference_valid'],
                  svg_match=valid and svg['exit']==0 and cmp['exit']==0,
                  panic=b'panicked at' in (d/'svg.stderr').read_bytes())
    try:
        projection = model((d/'yaml.stdout').read_bytes()); result['model'] = projection
        source = projection['source'].encode(); (d/'cli.source').write_bytes(source)
        if 'reference_source_capture' in row:
            result['exact_source'] = source == Path(row['reference_source_capture']).read_bytes()
    except (yaml.YAMLError, TypeError, ValueError, KeyError, AttributeError):
        result['model'] = None
    dump(d/'result.json', result)
    return result
def replay(phase):
    rows = json.loads((S/'membership.json').read_bytes())
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        results = list(pool.map(lambda row:measure(row,phase), rows))
    dump(S/(phase+'-results.json'),results)
    for suite in SUITES + ['maker13','maker2']:
        pairs = [(r,v) for r,v in zip(rows,results) if r['suite']==suite]
        print(suite,dict(entries=len(pairs),valid=sum(v['reference_valid'] for r,v in pairs),svg=sum(v['svg_match'] for r,v in pairs),
                         exact_source=sum(v.get('exact_source',False) and v['reference_valid'] for r,v in pairs),
                         available_source=sum('reference_source_capture' in r and v['reference_valid'] for r,v in pairs)),flush=True)
if __name__ == '__main__':
    if sys.argv[1] == 'freeze': freeze()
    else: replay(sys.argv[1])
