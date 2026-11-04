# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
from collections import Counter
import json,hashlib,xml.etree.ElementTree as ET
s=Path(__file__).resolve().parent;o=Path('/Users/marcelo/Library/Caches/rustuml-class-cff4')
before=json.loads((o/'corrected.json').read_text());after=json.loads((s/'inventory-v1.json').read_text());fresh=json.loads((s/'fresh-comparison.json').read_text());p36=json.loads((s/'focused-v1.json').read_text());base36=json.loads(Path('docs/parity-reviews/class-note-quark-modifier-lifecycle/maker-evidence/recovery4-remote36/comparison.json').read_text())
by={r['source']:r for r in after['results']};old={r['source']:r for r in before['results']}
oldcoh=json.loads(Path('docs/parity-reviews/class-note-quark-modifier-lifecycle/maker-evidence/recovery4/corrected-cohort-membership.json').read_text())
coh={k:dict(entries=len(ps),before=dict(Counter(old[p]['status'] for p in ps)),after=dict(Counter(by[p]['status'] for p in ps))) for k,ps in oldcoh.items()}
coh['maker36']={'entries':36,'before':dict(Counter('pass' if r['status']=='match' else r['status'] for r in base36['results'])),'after':p36['counts']}
coh['fresh20']={'entries':20,**fresh['counts']}
sha1559={r['source_sha256'] for r in before['results']};sha36={r['source_sha256'] for r in base36['results']};sha20={r['source_sha256'] for r in fresh['results']}
coh['overlap']={'1559_unique_source_hashes':len(sha1559),'36_unique_source_hashes':len(sha36),'20_unique_source_hashes':len(sha20),'1559_and_36_shared_source_hashes':len(sha1559&sha36),'fresh20_shared_with_preserved':len(sha20&(sha1559|sha36)),'interpretation':'1559 retains explicit duplicate entries. Cohort rows overlap internally (e.g. original810 is within955/1192/1449/1559); maker36 and fresh20 have no identical source bytes in1559 or each other.'}
(s/'cohorts.json').write_text(json.dumps(coh,indent=2)+'\n')
def elements(p):
 root=ET.parse(p).getroot();return [(e.tag,e.attrib,e.text) for e in root.iter() if e.tag.rsplit('}',1)[-1] not in ['g','a']]
def anchors(p):
 root=ET.parse(p).getroot();parents={c:par for par in root.iter() for c in par};return [{'parent_tag':parents[e].tag.rsplit('}',1)[-1],'parent_class':parents[e].get('class'),'attributes':e.attrib} for e in root.iter() if e.tag.endswith('}a')]
res=[]
for r in fresh['results']:
 if r['after']['status']=='pass':continue
 p=s/'fresh-pairs'/r['name'];same=elements(p.with_suffix('.before.svg'))==elements(p.with_suffix('.after.svg'));aa=anchors(p.with_suffix('.after.svg'));ja=anchors(p.with_suffix('.svg'))
 assert same and aa==ja
 res.append(dict(source=r['name'],before_status=r['before']['status'],after_status=r['after']['status'],non_group_anchor_elements_identical_before_after=same,after_and_java_anchor_parentage_identical=aa==ja,anchor_parentage=aa,scope='Existing group/edge geometry; URL ownership corrected but full image remains a mismatch. No geometry waiver or environment attribution.'))
measure=[]
for r in p36['results']:
 if '/boundary_' not in r['source']:continue
 p=Path(r['source']);ref=ET.parse(p.with_suffix('.svg')).getroot();current=ET.parse(s/'focused-v1'/p.with_suffix('.rust.svg')).getroot();measure.append(dict(source=str(p),java_canvas=ref.get('viewBox'),after_canvas=current.get('viewBox'),exact_canvas=ref.get('viewBox')==current.get('viewBox')))
assert all(r['exact_canvas'] for r in measure)
report={'fresh_residuals':res,'boundary36_canvas':measure,'unowned_glyph_failures':[r['source'] for r in p36['results'] if r['status']=='mismatch'],'preserved_process_errors':[{'source':r['source'],'exit':r['cli']['exit'],'stderr':r['cli']['stderr']} for r in after['results'] if r['status']=='process_error'],'lost_passes':[r['source'] for r in after['results'] if old[r['source']]['status']=='pass' and r['status']!='pass'],'new_errors':[r['source'] for r in after['results'] if old[r['source']]['status']!='process_error' and r['status']=='process_error'],'fresh_lost_passes':[r['name'] for r in fresh['results'] if r['before']['status']=='pass' and r['after']['status']!='pass'],'fresh_new_errors':[r['name'] for r in fresh['results'] if r['before']['status']!='process_error' and r['after']['status']=='process_error'],'model_retention':{'1559_ast_byte_identical':1559,'fresh20_yaml_byte_identical':sum(r['model_bytes_unchanged'] for r in fresh['results'])}}
(s/'residuals-and-invariants.json').write_text(json.dumps(report,indent=2)+'\n');print(coh['overlap']);print('Fresh group residuals preserve all non-container element data:',len(res))
