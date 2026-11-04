# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import base64,gzip,hashlib,json,os,subprocess,xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-review-b074');ROOT=Path.cwd()
sha=lambda b:hashlib.sha256(b).hexdigest()
r=json.loads((S/'supplement-reference-results.json').read_bytes());manifest=json.loads((S/'supplement-manifest.json').read_text()); expected={x['name']:x for x in manifest}
packed=(S/'supplement-reference-results.json.gz').read_bytes();transport=json.loads((S/'supplement-reference-transport.json').read_bytes())
assert sha(packed)=='9609277759983c64b8ff944f9aa86f37b33c812be5db1a4d586f91dfb97c471e'==transport['sha256']
assert base64.b64decode(transport['data'],validate=True)==packed and gzip.decompress(packed)==(S/'supplement-reference-results.json').read_bytes()
assert r['environment']['jar_sha256']=='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053'
assert r['environment']['input_compressed_sha256']==sha((S/'supplement-inputs.json.gz').read_bytes())
assert len(r['results'])==len(expected)==6 and {x['name'] for x in r['results']}==set(expected)
env=dict(os.environ,PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
results=[]
for row in r['results']:
    meta=expected[row['name']];src=Path(meta['source']); assert src.name==row['name'] and sha(src.read_bytes())==row['source_sha256']==meta['source_sha256']
    dst=S/'fresh'/meta['mechanism']/src.name;dst.parent.mkdir(parents=True,exist_ok=True);dst.write_bytes(src.read_bytes())
    for stream,suffix in [('stdout','.svg'),('stderr','.java.stderr')]:
        data=base64.b64decode(row[stream+'_base64'],validate=True); assert sha(data)==row[stream+'_sha256'];dst.with_suffix(suffix).write_bytes(data)
    assert row['timezone']=='GMT+11:00' and not row['timed_out'] and '-Djava.awt.headless=true' in row['java_argv']
    assert row['argv']==['/Users/marcelo/rustuml-reference/bin/reference','--cwd',row['cwd'],'--timezone','GMT+11:00','--','-pipe','-tsvg']
    ET.fromstring(dst.with_suffix('.svg').read_bytes())
    record=dict(name=src.name,mechanism=meta['mechanism'],axes=meta['axes'],source_sha256=meta['source_sha256'],reference_sha256=row['stdout_sha256'],java_exit=row['exit_code'])
    for variant in ['before','after']:
        records=[]
        cmds=[[str(S/f'target-{variant}/release/rustuml'),flag,str(src)] for flag in ['-tsvg','--yaml']]
        cmds.append([str(S/'check_svg'),str(dst.with_suffix('.svg')),str(dst.with_suffix(f'.{variant}.svg'))])
        for cmd,suffix in zip(cmds,[f'.{variant}.svg',f'.{variant}.yaml',f'.{variant}.diff']):
            p=subprocess.run(cmd,env=env,capture_output=True,timeout=60);dst.with_suffix(suffix).write_bytes(p.stdout);dst.with_suffix(suffix+'.stderr').write_bytes(p.stderr)
            records.append(dict(command=cmd,exit=p.returncode,stdout_sha256=sha(p.stdout),stderr_sha256=sha(p.stderr)))
        code=records[2]['exit'];status='invalid_reference' if code==2 else 'process_error' if records[0]['exit'] else 'pass' if code==0 else 'mismatch' if code==1 else 'comparison_error'
        record[variant]=dict(status=status,runs=records)
    record['model_unchanged']=dst.with_suffix('.before.yaml').read_bytes()==dst.with_suffix('.after.yaml').read_bytes();results.append(record)
report=dict(reference_validation='all 6 membership, input/stream/archive hashes, exact wrapper argv/cwd/timezone/headless pinned JAR validated; XML parsed; frozen marker policy classifies reference separately',results=results,counts={v:dict(Counter(x[v]['status'] for x in results)) for v in ['before','after']})
(S/'evidence/supplement6.json').write_text(json.dumps(report,indent=2)+'\n');print(report['counts']);print([(x['name'],x['after']['status']) for x in results]);print('models retained',sum(x['model_unchanged'] for x in results))
