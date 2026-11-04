# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures, gzip, hashlib, json, os, subprocess, sys, time
from collections import Counter
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-review-b074'); ROOT=Path.cwd()
P=ROOT/'docs/parity-reviews/class-simple-symbol-painted-maximum/maker-evidence/packet5'
ENV=dict(os.environ,PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100',RAYON_NUM_THREADS='4')
def sha(data):return hashlib.sha256(data).hexdigest()
def run(cmd):
    try:
        p=subprocess.run(cmd,env=ENV,capture_output=True,timeout=60); return p.returncode,p.stdout,p.stderr
    except subprocess.TimeoutExpired as e:return 124,e.stdout or b'',(e.stderr or b'')+b'REVIEW TIMEOUT'
variant=sys.argv[1]; binary=S/f'target-{variant}/release/rustuml'
cohorts={name:json.loads(gzip.decompress((P/file).read_bytes()))['results'] for name,file in [('preserved1559','inventory-v1.json.gz'),('prior36','focused-v1.json.gz')]}
cohorts['maker20']=[dict(source=str((P/'fresh-pairs'/r['name']).relative_to(ROOT)),source_sha256=r['source_sha256'],java_sha256=r['reference_sha256']) for r in json.loads((P/'fresh-comparison.json').read_text())['results']]
for cohort,rows in cohorts.items():
    started=time.time(); dest=S/'captures'/variant/cohort;dest.mkdir(parents=True,exist_ok=True)
    def one(item):
        i,r=item;source=ROOT/r['source'];ref=source.with_suffix('.svg')
        assert sha(source.read_bytes())==r['source_sha256']; assert sha(ref.read_bytes())==r['java_sha256']
        out=dest/f'{i:04d}'; records=[]
        cmds=[[str(binary),'-tsvg',str(source)],[str(binary),'--ast',str(source)],[str(S/'check_svg'),str(ref),str(out.with_suffix('.svg'))]]
        for cmd,suffix in zip(cmds,['.svg','.ast','.diff']):
            code,stdout,stderr=run(cmd);out.with_suffix(suffix).write_bytes(stdout);out.with_suffix(suffix+'.stderr').write_bytes(stderr)
            records.append(dict(command=cmd,exit=code,stdout_sha256=sha(stdout),stderr_sha256=sha(stderr),stderr=stderr.decode(errors='replace')))
        c,a,d=records; status='invalid_reference' if d['exit']==2 else 'process_error' if c['exit'] else 'pass' if d['exit']==0 else 'mismatch' if d['exit']==1 else 'comparison_error'
        return dict(index=i,source=r['source'],source_sha256=r['source_sha256'],reference_sha256=r['java_sha256'],maker_status=r.get('status'),status=status,runs=records)
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:results=list(pool.map(one,enumerate(rows)))
    report=dict(variant=variant,revision='c1a38ada817d820ca8f57b1e8db0d9048e9657eb' if variant=='before' else 'b074d2082cb9ed2b9460f79a32b170e31cc1ab5e',binary=str(binary),binary_sha256=sha(binary.read_bytes()),cwd=str(ROOT),environment={k:ENV[k] for k in ['PLANTUML_STDLIB','RUSTUML_DEBUG','RAYON_NUM_THREADS']},counts=dict(Counter(r['status'] for r in results)),seconds=time.time()-started,results=results)
    (S/'evidence'/f'{cohort}-{variant}.json.gz').write_bytes(gzip.compress((json.dumps(report,indent=2)+'\n').encode(),mtime=0));print(cohort,variant,report['counts'],flush=True)
