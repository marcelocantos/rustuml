# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,difflib,hashlib,json,xml.etree.ElementTree as ET
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();P=R/'docs/parity-reviews/sequence-command-admission-packet/maker-results'
sha=lambda b:hashlib.sha256(b).hexdigest()
def write(name,data):(C/name).write_text(json.dumps(data,indent=2)+'\n')
summary={};all_changes=[];xml=[]
maker_changes=json.loads((P/'model-changes.json').read_text())
for cohort in ['retained','golden']:
    before=json.loads((C/(cohort+'-before-full/results.json')).read_text());after=json.loads((C/(cohort+'-after-full/results.json')).read_text());assert len(before)==len(after)
    def counts(rows):
        totals=collections.Counter()
        for r in rows:
            code=r['comparison']['exit'];cli=r['results']['svg']['exit']
            key='nonpositive' if code==2 else 'reference_read_failure' if code==101 else 'valid_error' if cli else 'match' if code==0 else 'mismatch'
            totals[key]+=1
        return dict(entries=len(rows),unique_sources=len({r['source_sha256'] for r in rows}),counts=dict(totals),cli_panics=[r['source'] for r in rows if r['results']['svg']['exit'] not in (0,1)],timeouts=[r['source'] for r in rows if r['results']['svg']['exit']==124])
    gains=[];losses=[];errors=[];lost_rejections=[];models={}
    for x,y in zip(before,after):
        assert x['source']==y['source'] and x['source_sha256']==y['source_sha256']
        old=x['results']['svg'];new=y['results']['svg']
        if x['comparison']['exit']==0 and old['exit']==0 and (y['comparison']['exit']!=0 or new['exit']):losses.append(y['source'])
        if y['comparison']['exit']==0 and new['exit']==0 and x['comparison']['exit']!=0:gains.append(y['source'])
        if old['exit']==0 and new['exit']!=0 and x['comparison']['exit'] not in (2,101):errors.append(dict(source=y['source'],source_sha256=y['source_sha256'],diagnostic=new['stderr']))
        if old['exit']!=0 and new['exit']==0:lost_rejections.append(y['source'])
        for side,row in [('before',x),('after',y)]:
            raw=(C/(cohort+'-'+side+'-full')/row['results']['svg']['stdout_file']).read_bytes()
            if raw:
                try:root=ET.fromstring(raw);xml.append(dict(cohort=cohort,side=side,source=row['source'],valid=True,family=root.attrib.get('data-diagram-type')))
                except ET.ParseError as e:xml.append(dict(cohort=cohort,side=side,source=row['source'],valid=False,error=str(e),cli_exit=row['results']['svg']['exit']))
    for mode in ['ast','ascii','yaml']:
        changed=0;new_errors=0;diagnostic=0
        for x,y in zip(before,after):
            old=x['results'][mode];new=y['results'][mode];new_errors+=old['exit']==0 and new['exit']!=0;diagnostic+=old['stderr']!=new['stderr']
            if old['stdout_sha256']==new['stdout_sha256']:continue
            changed+=1;raw_old=(C/(cohort+'-before-full')/old['stdout_file']).read_text();raw_new=(C/(cohort+'-after-full')/new['stdout_file']).read_text()
            diff=''.join(difflib.unified_diff(raw_old.splitlines(True),raw_new.splitlines(True),fromfile='before',tofile='after'))
            if old['exit']==0 and new['exit']!=0:reason='Command admission/color execution now rejects; exact new diagnostic retained.'
            elif old['exit']!=0 or new['exit']!=0:reason='Error-mode serialization differs; full exit and diagnostic retained independently of SVG.'
            elif 'footbox' in (R/x['source']).read_text().lower():reason='Footbox command consumption changes visibility and removes fabricated messages/participants.'
            elif 'legend_line' in diff or 'source_line' in diff:reason='Selected registered multiline command owns source-line metadata; legend start is recorded at its opening command, not body line.'
            elif 'participant' in (R/x['source']).read_text() and '[' in (R/x['source']).read_text():reason='Registered unsupported participant block is skipped with explicit class warning instead of fabricated body events.'
            else:reason='Inspect full diff; potential map iteration order or uncategorized semantic change.'
            all_changes.append(dict(cohort=cohort,mode=mode,source=x['source'],source_sha256=x['source_sha256'],before_exit=old['exit'],after_exit=new['exit'],before_stderr=old['stderr'],after_stderr=new['stderr'],reason=reason,diff=diff))
        models[mode]=dict(entries=len(before),stdout_changes=changed,new_errors=new_errors,diagnostic_changes=diagnostic)
    summary[cohort]=dict(before=counts(before),after=counts(after),gains=gains,lost_matches=losses,new_valid_errors=errors,lost_rejections=lost_rejections,model_changes=models)
exceptions=json.loads((P/'parent-approved-exceptions.json').read_text());expected={x['source_sha256'] for x in exceptions};observed={x['source_sha256'] for x in summary['retained']['new_valid_errors']}
summary['exception_set']=dict(expected=sorted(expected),observed=sorted(observed),equal=expected==observed,extra=sorted(observed-expected),missing=sorted(expected-observed))
summary['strict_xml']=dict(outputs=len(xml),failures=[r for r in xml if not r['valid']],sequence_outputs=sum(r.get('family')=='SEQUENCE' for r in xml))
write('summary.json',summary);write('model-changes.json',all_changes);write('strict-xml.json',xml)
print(json.dumps({k:v for k,v in summary.items() if k not in ('strict_xml','exception_set')},indent=2)[:8000])
