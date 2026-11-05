# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Retain the final four attempts, including failures; revise only this checker's records."""
from pathlib import Path
from collections import Counter
import hashlib
import json
import shutil

root=Path.cwd()
tmp=Path('/tmp/rustuml-checker2740')
support=root/'docs/parity-reviews/class-command-grammar-closure/checker-support'
rows=json.loads((tmp/'acceptance-results.json').read_text())['results']
meta={v['source']:v for v in json.loads((tmp/'generation-acceptance.json').read_text())}
manifest=json.loads((support/'retained-manifest.json').read_text())
for r in rows:
    src=Path(r['source']);m=src.parent.name
    dest=root/'test-diagrams/perturbations'/m/'checker'/src.name
    assert not dest.exists()
    shutil.copyfile(src,dest);shutil.copyfile(src.with_suffix('.svg'),dest.with_suffix('.svg'))
    entry={'source':str(dest.relative_to(root)),'golden':str(dest.with_suffix('.svg').relative_to(root)),'axes':meta[r['source']]['axes'],'result':'pass' if r['status']=='pass' else 'fail','observed_status':r['status'],'java_valid':r['status']!='invalid_reference','origin':'final independent source-guided attempt to break a narrowly passing identity mechanism','original_source':r['source'],'source_sha256':r['source_sha256'],'java_sha256':r['java_sha256'],'cli_exit':r['cli']['exit'],'strict_comparator_exit':r['comparison']['exit'],'evidence_output':r['output'].replace(str(tmp)+'/','candidate-outputs.tar.gz::')}
    manifest.append(entry)
    p=root/'docs/parity-reviews'/m/'review.json';review=json.loads(p.read_text())
    review['perturbations'].append(entry)
    review['observed_counts']=dict(Counter(e['observed_status'] for e in review['perturbations']))
    if m=='class-quoted-code-identity':
        review['verdict']='rejected'
        review['findings']=[
            'Canonical quoted identity is preserved across declarations/links/notes in the source model, and four initial full-CLI cases pass. However the additional dotted-code and LR/note perturbations both fail strict CLI comparison, so this mechanism is not accepted.',
            'quoted_dotted creates an implicit namespace. Java emits the root-scope note before the nested leaf; Rust emits the leaf before that note despite retaining the same quark IDs. quoted_lr_order retains identity and declarations but places a bottom note to the right under LR direction. These are concrete emission/note-layout dependencies, not evidence that character preservation itself is wrong.',
            'The complete combined revision alone was tested. No production subset is approved for extraction. The original quoted-code account is preserved; this checker does not alter any historical review.'
        ]
        review['dependencies']=['class-note-declaration-scope','implicit namespace entity/note emission ordering','LR note direction translation']
    else:
        review['findings'][0]=review['findings'][0].replace('Six fresh','Eight fresh')+' Final probes also cover dotted aliases, command-looking/bracketed displays and reopened nested LR clouds.'
    if 'python3 /tmp/checker2740_acceptance_probes.py' not in review['commands']:review['commands'].append('python3 /tmp/checker2740_acceptance_probes.py')
    p.write_text(json.dumps(review,indent=2)+'\n')
(support/'retained-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
verdicts=json.loads((support/'verdicts.json').read_text())
for v in verdicts:
    review=json.loads((root/'docs/parity-reviews'/v['mechanism']/'review.json').read_text())
    v['verdict']=review['verdict'];v['counts']=review['observed_counts']
(support/'verdicts.json').write_text(json.dumps(verdicts,indent=2)+'\n')
for name in ['acceptance-results.json','generation-acceptance.json']:
    shutil.copyfile(tmp/name,support/name)
for name in ['checker2740_acceptance_probes.py','checker2740_append.py']:
    shutil.copyfile(Path('/tmp')/name,support/'scripts'/name)
print(len(manifest),Counter(v['verdict'] for v in verdicts))
