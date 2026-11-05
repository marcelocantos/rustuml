# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Classification-only refinement; does not execute either product or comparator."""
import json,collections,xml.etree.ElementTree as ET
from pathlib import Path
S=Path('/Users/marcelo/Library/Caches/rustuml-unresolved-sprite-checker')
rs=json.loads((S/'fresh-results.json').read_bytes());rows=[]
for r in rs:
 d=S/'fresh-reference'/r['name'];b=(d/'java.svg.stdout').read_bytes();err=(d/'java.svg.stderr').read_text();text=' '.join(ET.fromstring(b).itertext()) if b else ''
 category=r['classification']
 if 'java.lang.NumberFormatException' in err:category='runtime_error_svg'
 elif 'Empty description' in err and 'Welcome to PlantUML!' in text:category='empty_description_welcome_error_svg'
 elif 'Syntax Error?' in err:category='syntax_error_svg_with_welcome'
 elif 'Welcome to PlantUML!' in text:category='welcome_svg'
 rows.append(dict(name=r['name'],intended_validity=r['row']['intended_validity'],observed_validity=category,positive_reference=r['reference_valid'],initial_mechanical_category=r['classification'],evidence=dict(java_exit=r['reference_exit'],comparator_exit=r['reference_comparator_exit'],stderr=err[:180],xml_valid=r['reference_xml_valid']),before_match=r['phases']['before']['svg_match'],after_match=r['phases']['after']['svg_match'],standing_match=r['standing']['exit']==0))
summary=dict(entries=len(rows),observed_categories=dict(collections.Counter(x['observed_validity'] for x in rows)),valid=sum(r['positive_reference'] for r in rows),before_pass=sum(x['before_match'] for x in rows),after_pass=sum(x['after_match'] for x in rows),standing_pass=sum(x['standing_match'] for x in rows),lost_passes=[x['name'] for x in rows if x['before_match'] and not x['after_match']],new_passes=[x['name'] for x in rows if not x['before_match'] and x['after_match']],positive_failures=[x['name'] for x in rows if x['positive_reference'] and not x['after_match']],model_changes=[r['name'] for r in rs if r['phases']['before']['model']!=r['phases']['after']['model']],zero_byte_java_outputs=sum(not (S/'fresh-reference'/r['name']/'java.svg.stdout').read_bytes() for r in rs))
(S/'fresh-classification.json').write_text(json.dumps(rows,indent=2)+'\n');(S/'fresh-summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))
# Preserve observable text/geometry; these are diagnostics, never the whole-SVG acceptance gate.
observations=[]
for r in rs:
 views={}
 for phase,path in [('java',S/'fresh-reference'/r['name']/'java.svg.stdout'),('before',S/'replay/before/checker38'/r['name']/'svg.stdout'),('after',S/'replay/after/checker38'/r['name']/'svg.stdout')]:
  b=path.read_bytes()
  try:
   root=ET.fromstring(b);views[phase]=dict(root=dict(root.attrib),text=[dict(text=''.join(n.itertext()),attributes=dict(n.attrib)) for n in root.iter() if n.tag.rsplit('}',1)[-1]=='text'],images=[dict(n.attrib) for n in root.iter() if n.tag.rsplit('}',1)[-1]=='image'])
  except ET.ParseError:views[phase]=dict(xml_valid=False,bytes=len(b))
 observations.append(dict(name=r['name'],views=views))
(S/'text-observations.json').write_text(json.dumps(observations,indent=2)+'\n')
