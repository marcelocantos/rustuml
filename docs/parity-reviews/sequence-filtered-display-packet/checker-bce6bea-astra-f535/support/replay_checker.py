# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures,hashlib,importlib.util,json,os,subprocess,sys
from pathlib import Path
C=Path(__file__).parent/'independent';R=Path.cwd();label=sys.argv[1];cohort=sys.argv[2] if len(sys.argv)>2 else 'packet2371'
manifest=C/(cohort+'.json')
if cohort=='packet2371':
 src=R/'docs/parity-reviews/sequence-filtered-display-packet/maker-results/all-inputs.json'
 manifest.write_bytes(src.read_bytes())
binaries=C/(label+'-bin');support=R/'docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py'
spec=importlib.util.spec_from_file_location('unchanged_measure',support);measure=importlib.util.module_from_spec(spec);spec.loader.exec_module(measure)
measure.measure(manifest,binaries,C/(cohort+'-'+label),R)
def capture(item):
 body=(R/item['source']).read_bytes();row=dict(item,source_sha256=hashlib.sha256(body).hexdigest(),results={})
 for mode,flags in [('model',['--ast','-pipe']),('ascii',['-ttxt','-pipe']),('yaml',['--yaml','-pipe'])]:
  argv=[str(binaries/'rustuml'),*flags];p=measure.run(argv,R,body);row['results'][mode]=dict(p,command=argv,cwd=str(R))
 return row
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(capture,json.loads(manifest.read_text())))
(C/(cohort+'-'+label+'-models.json')).write_text(json.dumps(rows,indent=2)+'\n');print('models captured',label,len(rows),flush=True)
