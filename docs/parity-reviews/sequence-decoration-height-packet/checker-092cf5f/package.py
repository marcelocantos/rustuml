# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections, gzip, hashlib, json, shutil, subprocess, tarfile
from pathlib import Path

cache=Path(__file__).parent
root=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-607d50f7')
packet=root/'docs/parity-reviews/sequence-decoration-height-packet/checker-092cf5f'
packet.mkdir(exist_ok=False)
def sha(b): return hashlib.sha256(b).hexdigest()
def put(path,value): path.write_text(json.dumps(value,indent=2)+'\n')
def compress(source,destination): destination.write_bytes(gzip.compress(source.read_bytes(),mtime=0))
def mechanism(name):
    if name.startswith('child_'): return 'sequence-ordinary-note-child-allocation'
    if name.startswith('rule_'): return 'sequence-note-rule-title-height'
    if name.startswith('underline_'): return 'sequence-message-text-component-height'
    if name.startswith('style_'): return 'sequence-component-style-consumption'
    return 'sequence-target-circle-decoration'
requests={r['name']:r for r in json.loads((cache/'reference-manifest.json').read_text())['cases']}
refs={r['name']:r for r in json.loads((cache/'reference-results.json').read_text())['results']}
classes={r['name']:r for r in json.loads((cache/'reference-classification.json').read_text())}
before=json.loads((cache/'fresh-before/measurements.json').read_text())
after=json.loads((cache/'fresh-after/measurements.json').read_text())
assert len(before)==len(after)==len(requests)==31
sources=[];reviews=collections.defaultdict(list)
for old,new in zip(before,after):
    name=new['name']; assert old['name']==name
    mech=mechanism(name); directory=root/'test-diagrams/perturbations'/mech;directory.mkdir(exist_ok=True)
    src=directory/('checker_092_'+name+'.puml');gold=src.with_suffix('.svg')
    assert not src.exists() and not gold.exists()
    src.write_bytes(Path(requests[name]['source']).read_bytes());gold.write_bytes(Path(new['golden']).read_bytes())
    src.with_suffix('.stderr').write_bytes(Path(new['golden']).with_suffix('.stderr').read_bytes())
    for mode,row in [('before',old),('after',new)]:
        src.with_suffix('.'+mode+'.rust.svg').write_bytes((cache/('fresh-'+mode)/row['cli']['svg']).read_bytes())
        src.with_suffix('.'+mode+'.stderr').write_text(row['cli']['stderr'])
    provenance=dict(reference={k:v for k,v in refs[name].items() if not k.endswith('_base64')},request=requests[name],classification=classes[name],source=str(src.relative_to(root)),golden=str(gold.relative_to(root)),before=old,after=new)
    put(src.with_suffix('.provenance.json'),provenance)
    result='pass' if new['cli_comparison']['exit']==0 and new['cli']['exit']==0 and classes[name]['classification']=='valid_sequence' else 'fail'
    row=dict(source=str(src.relative_to(root)),golden=str(gold.relative_to(root)),cohort='checker_092_fresh31',name=name,axes=requests[name]['axes'],result=result,reference_classification=classes[name]['classification'],source_sha256=sha(src.read_bytes()),golden_sha256=sha(gold.read_bytes()),cli_exit=new['cli']['exit'],comparator_exit=new['cli_comparison']['exit'])
    sources.append(row);reviews[mech].append(row)
put(packet/'pairs.json',sources)
put(packet/'fresh-inputs.json',[{k:v for k,v in r.items() if k in ['source','golden','cohort','name','axes']} for r in sources])
summary={}
for name,rows in [('before',before),('after',after)]:
    summary[name]=dict(entries=len(rows),mode_exits={mode:dict(collections.Counter(r[mode]['exit'] for r in rows)) for mode in ['cli_comparison','product_library','assisted_library']},valid_cli_errors=[r['name'] for r in rows if classes[r['name']]['classification']=='valid_sequence' and r['cli']['exit']!=0],negative_status=[dict(name=r['name'],java_exit=refs[r['name']]['exit_code'],cli_exit=r['cli']['exit'],classification=classes[r['name']]['classification']) for r in rows if classes[r['name']]['classification']!='valid_sequence'])
summary['recoveries']=[n['name'] for o,n in zip(before,after) if o['cli_comparison']['exit']!=0 and n['cli_comparison']['exit']==0]
summary['losses']=[n['name'] for o,n in zip(before,after) if o['cli_comparison']['exit']==0 and n['cli_comparison']['exit']!=0]
summary['unique_source_strings']=31;summary['overlap_with_frozen']=0
summary['per_mechanism']={k:dict(entries=len(v),valid_matches=sum(r['result']=='pass' for r in v),valid_mismatches=sum(r['reference_classification']=='valid_sequence' and r['result']=='fail' for r in v),invalid_or_empty=sum(r['reference_classification']!='valid_sequence' for r in v)) for k,v in reviews.items()}
put(packet/'fresh-summary.json',summary)
for name in ['before-build.json','intermediate-build.json','after-build.json','checks.json','summary.json','main-failure-sets.json','environment.json','model-retention.json','ascii.json','reference-manifest.json','execution-cwd-mapping.json','reference-classification.json','parent-relay-validation.json','setup-rejection.json','transfer-provenance.json']:
    shutil.copyfile(cache/name,packet/name)
for name in ['audit.json','frozen-inventory.json','reference-transport.json']:
    compress(cache/name,packet/(name+'.gz'))
for name in ['reference-results.json.gz','reference-input-bundle.json.gz','transfer-roundtrip.json.gz']:
    shutil.copyfile(cache/name,packet/name)
for name in ['build.py','validate.py','generate.py','audit.py','diagnostics.py','import_reference.py','model_retention.rs','package.py']:
    shutil.copyfile(cache/name,packet/name)
for mode in ['frozen-before','frozen-after','fresh-before','fresh-after']:
    compress(cache/mode/'measurements.json',packet/(mode+'.json.gz'))
logs=packet/'logs';logs.mkdir()
for p in cache.glob('*.log'): compress(p,logs/(p.name+'.gz'))
for p in cache.glob('*failure_names.txt'): compress(p,logs/(p.name+'.gz'))
shutil.copytree(cache/'main-regression',packet/'main-regression')
shutil.copytree(cache/'asts',packet/'asts')
with tarfile.open(packet/'frozen-cli-captures.tar.gz','w:gz') as tar:
    for name in ['frozen-before','frozen-after']:
        for p in sorted((cache/name).glob('*.svg')): tar.add(p,arcname=name+'/'+p.name)
print(json.dumps(summary,indent=2))
