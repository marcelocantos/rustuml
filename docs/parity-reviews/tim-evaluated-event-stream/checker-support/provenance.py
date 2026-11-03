# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Audit immutable historical blobs and annotate the reader reproduction."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import xml.etree.ElementTree as ET

path=Path(__file__).resolve().parent/'check.py'
spec=importlib.util.spec_from_file_location('check',path)
m=importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
historical=subprocess.check_output(['git','rev-parse','97ac6290']).decode().strip()
data={'historical_repository_commit':historical,'golden_gitlink':subprocess.check_output(['git','-C','test-diagrams/golden','rev-parse','HEAD']).decode().strip(),'generator_blobs':[],'frozen_pairs':[]}
for name in ['gen_activity.py','gen_edge_cases.py','render_all.py']:
    file='test-diagrams/golden/'+name
    blob=subprocess.check_output(['git','rev-parse',historical+':'+file]).decode().strip()
    content=subprocess.check_output(['git','cat-file','blob',blob])
    data['generator_blobs'].append({'path':file,'git_blob':blob,'sha256':hashlib.sha256(content).hexdigest()})
results=json.loads((m.SUPPORT/'reader-contract/results.json').read_text())
for row in results:
    source=Path(row['source'])
    out=m.SUPPORT/'reader-contract'/source.stem
    (out/'input.svg').write_bytes((out/'pipe.svg').read_bytes())
    (out/'decoded.svg').write_bytes((out/'decoded-pipe.svg').read_bytes())
    raw=source.read_bytes()
    r=m.run([m.BIN,'-pipe'],raw)
    (out/'rust.svg').write_bytes(r.stdout)
    row['rust_exit']=r.returncode
    row['rust_comparisons']={}
    row['source_lines']={}
    row['filter_ids']={}
    for name,golden in [('frozen',source.with_suffix('.svg')),('pipe',out/'pipe.svg'),('get',out/'get.svg')]:
        c=m.run([m.SCRATCH/'probe','compare',golden,out/'rust.svg'])
        row['rust_comparisons'][name]=c.returncode
        (out/('rust-vs-'+name+'.txt')).write_bytes(c.stdout+c.stderr)
    for name in ['pipe','get','file','post','rust']:
        xml=ET.fromstring((out/(name+'.svg')).read_bytes())
        row['source_lines'][name]=[e.attrib['data-source-line'] for e in xml.iter() if 'data-source-line' in e.attrib]
        row['filter_ids'][name]=[e.attrib['id'] for e in xml.iter() if e.tag.endswith('}filter')]
    if 'golden' in source.parts:
        entries=[]
        for path in [source,source.with_suffix('.svg')]:
            old=subprocess.check_output(['git','show',historical+':'+str(path)])
            current=path.read_bytes()
            entries.append({'path':str(path),'historical_git_blob':subprocess.check_output(['git','rev-parse',historical+':'+str(path)]).decode().strip(),'sha256':hashlib.sha256(current).hexdigest(),'historical_bytes_equal_current':old==current})
        data['frozen_pairs'].extend(entries)
data['reader_results']=results
data['limits']=['No per-file historical HTTP access logs survive in this checkout; route attribution uses the preserved renderer program, byte-identical source/SVG history, and exact parsed-SVG reproduction by the pinned Java GET route.', 'Rebuilt JAR is source-pinned, not historical-byte-identical; broad font/Graphviz equivalence is not claimed.']
(m.SUPPORT/'provenance-results.json').write_text(json.dumps(data,indent=2)+'\n')
print(json.dumps(data,indent=2))
