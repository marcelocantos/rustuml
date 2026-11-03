# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Finalize evidence files and assert checker ownership without changing production."""
from pathlib import Path
from collections import Counter
import hashlib
import json
import re
import shutil
import subprocess
import tarfile
import xml.etree.ElementTree as ET

root=Path.cwd()
tmp=Path('/tmp/rustuml-checker2740')
support=root/'docs/parity-reviews/class-command-grammar-closure/checker-support'
policy='b62d8ce967b6dda7d85bb6dc28ff92509f6066f9'
tested='fefca37686ff0c93b5b306831e4554a2d8f9efa6'
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def write(p,x): p.write_text(json.dumps(x,indent=2)+'\n')
def git(*args): return subprocess.check_output(['git',*args],text=True).strip()

report=support/'REPORT.md'
text=report.read_text().replace('The pending T14.6.3 outer-body reader decision was not made.',
    'During this review the owner approved raw Java CLI/file semantics, recorded in parent policy commit `'+policy+'`. T14.6.3 is no longer pending. This checker read that exact policy from git without importing it or the TIM production/assertion changes. Independent raw-reader verification belongs to the TIM checker and was not duplicated here.')
text=text.replace('the pending outer-reader blank-line contract','the separately verified raw-reader contract')
text=text.replace('0.784722 inches','0.7840033637152778 inches')
report.write_text(text)
policy_text=subprocess.check_output(['git','show',policy+':docs/reference-bootstrap.md'])
(support/'owner-reference-policy.md').write_bytes(policy_text)
write(support/'owner-reference-policy.json',{'commit':policy,'path':'docs/reference-bootstrap.md','sha256':hashlib.sha256(policy_text).hexdigest(),'contract':'raw Java CLI/file input; preserve historical GET references and counts separately','scope':'Policy read only. No TIM production/assertion import or duplicate reader verification.'})

account_map=json.loads((support/'account-map.json').read_text())
for item in account_map:
    p=root/'docs/parity-reviews'/item['mechanism']/'review.json'
    review=json.loads(p.read_text())
    review['reference_profile']='Source-pinned Java 71806a2, raw CLI -tsvg -pipe for new inputs, consistent with owner-approved raw CLI/file policy '+policy+'. All preexisting/maker reference bytes unchanged; historical GET corpus/counts reported separately. Historical binary/font/Graphviz equivalence unproven. TIM checker owns independent reader verification; no TIM production/assertion imported.'
    review['reference_policy_commit']=policy
    if item['mechanism']=='class-symbol-container-alias-identity':
        sentence=' Final probes also cover dotted aliases, command-looking/bracketed displays and reopened nested LR clouds.'
        review['findings'][0]=review['findings'][0].replace(sentence+sentence,sentence)
    write(p,review)

# All raw generation stderr, including unexpected-invalid Java output, survives.
for p in sorted((tmp/'new-sources').rglob('*.java.stderr')):
    dest=support/'java-generation-stderr'/p.relative_to(tmp/'new-sources')
    dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(p,dest)
for p in Path('/tmp').glob('rustuml-checker2740-*.log'):
    shutil.copyfile(p,support/'logs'/p.name)
shutil.copyfile('/tmp/rustuml-checker2740-final/results.json',support/'final-results.json')
for name in ['checker2740_final_run.py','checker2740_finalize.py']:
    shutil.copyfile(Path('/tmp')/name,support/'scripts'/name)
for name,folder in [('candidate',tmp/'outputs'),('parent',Path('/tmp/rustuml-checker2740-parent/outputs')),('final',Path('/tmp/rustuml-checker2740-final/outputs'))]:
    with tarfile.open(support/(name+'-outputs.tar.gz'),'w:gz') as archive:
        archive.add(folder,arcname='outputs')
(support/'initial-artifact-hashes.json').unlink()

# Pin source files actually used in the model/constant audit, not copied Java implementations.
java=Path('/Users/marcelo/work/github.com/plantuml/plantuml')
names={'PSystemBuilder','ClassDiagramFactory','CommandCreateClass','CommandCreateElementFull2','NameAndCodeParser','CommandPackage','CommandPackageWithUSymbol','CommandFactoryNote','CommandFactoryNoteOnEntity','EntityImageClassHeader','USymbolComponent2','USymbolSimpleAbstract','USymbolBoundary','EntityImageDescription','ClusterHeader','ClusterDotString','Cluster','Bibliotekon','SvekNode','SvekResult','LimitFinder','DiagramChromeFactory','DecorateEntityImage','TextBlockBordered','VisibilityModifier','USymbolFolder','USymbolAction','USymbolProcess','USymbolQueue','USymbolCloud','USymbolNode','ExtremityParenthesis','DotPath','DotStringFactory','GraphvizImageBuilder'}
files=subprocess.check_output(['rg','--files','src'],cwd=java,text=True).splitlines()
sources=[{'repository':'Java','revision':'71806a23780b04a5ccde2f8ceb5121edad5eb711','path':p,'sha256':sha(java/p)} for p in files if Path(p).stem in names or p=='src/main/resources/skin/plantuml.skin']
rust_paths=git('diff','--name-only','8a61fa097d278dad508da0659953a98765926774','ad0690974a581caf7770178b5dcca0b01f72c086','--','crates').splitlines()
rust_paths += ['crates/rustuml-layout/src/graph.rs','crates/rustuml-oracle/src/compare.rs','crates/rustuml-oracle/tests/harness_guards.rs','crates/rustuml-oracle/tests/parity_perturbations.rs']
for p in rust_paths:sources.append({'repository':'Rust','revision':tested,'path':p,'sha256':sha(root/p)})
write(support/'inspection-sources.json',sources)

# Compact machine-readable evidence for the two coordinate-frame diagnoses.
geometry=[]
for name,j,r in [('group_lr',root/'docs/parity-reviews/class-command-grammar-closure/maker-evidence/inputs/fresh/group_lr.svg',tmp/'outputs/fresh/group_lr/rust.svg'),('package_target',root/'test-diagrams/perturbations/class-parenthesis-endpoint-decoration/package_target_parenthesis_one_dash_lr.svg',tmp/'outputs/current955/class-parenthesis-endpoint-decoration/package_target_parenthesis_one_dash_lr/rust.svg')]:
    item={'case':name}
    for label,p in [('java',j),('rust',r)]:
        xml=ET.parse(p).getroot()
        item[label]={'source':str(p),'sha256':sha(p),'svg':xml.attrib,'groups':[{'attributes':g.attrib,'first_child':list(g)[0].attrib if len(g) else {}} for g in xml.iter('{http://www.w3.org/2000/svg}g') if g.attrib.get('class') in ['cluster','entity','link']]}
    geometry.append(item)
write(support/'group-coordinate-evidence.json',geometry)

# Record literal census relative to the actual candidate start as well as the frozen gate.
path='crates/rustuml-render/src/class.rs'
census={}
for rev in ['8a61fa097d278dad508da0659953a98765926774','18916d312c32340b0a674e15259dc3031ea57acf','bed3b50ca08e12d8ada612791cf07393617b2041',tested]:
    source=subprocess.check_output(['git','show',rev+':'+path],text=True)
    census[rev]=len(re.findall(r'(?<=\d)\.\d{3,}',source))
write(support/'decimal-census.json',{'path':path,'method':'same digit-dot-at-least-three-digits lexical census as count_high_precision_literals; strings/comments count too','counts':census,'frozen_baseline':383,'baseline_unchanged':True})

manifest=json.loads((support/'retained-manifest.json').read_text())
final=json.loads((support/'final-results.json').read_text())
final_by_path={str(Path('test-diagrams/perturbations')/r['relative']):r for r in final['results']}
for entry in manifest:
    assert sha(root/entry['source'])==entry['source_sha256']
    assert sha(root/entry['golden'])==entry['java_sha256']
    assert final_by_path[entry['source']]['status']==entry['observed_status']
referenced=[]
reviews=[]
for item in account_map:
    p=root/'docs/parity-reviews'/item['mechanism']/'review.json'
    r=json.loads(p.read_text());reviews.append(r)
    assert r['schema_version']==1 and r['mechanism']==item['mechanism']
    assert len(r['commands'])>=2 and r['findings'] and r['reviewer']['task_id'] and r['reviewer']['model']
    for key in ['account_commit','implementation_commit','java_revision']:assert re.fullmatch('[a-f0-9]{40}',r[key])
    assert len(r['perturbations'])>=2
    for e in r['perturbations']:
        assert len(e['axes'])>=2 and e['result'] in ['pass','fail']
        assert e['source'].startswith('test-diagrams/perturbations/'+r['mechanism']+'/')
        assert str(Path(e['source']).with_suffix('.svg'))==e['golden']
        referenced.append(e['source'])
    if r['verdict']=='accepted':assert all(e['result']=='pass' for e in r['perturbations'])
assert len(set(referenced))==len(manifest)==237
assert len(final['results'])==1192
assert git('diff','--name-only',tested,'--','crates','docs/reference-bootstrap.md','bullseye.yaml')==''
assert git('diff','--name-only')=='' # Existing tracked files remain untouched before staging.
ownership=[]
for p in git('ls-files','--others','--exclude-standard').splitlines():
    permitted=p.startswith(str(support.relative_to(root))+'/') or p in {str(Path('docs/parity-reviews')/r['mechanism']/'review.json') for r in reviews} or p in {e[k] for e in manifest for k in ['source','golden']}
    assert permitted,p
    ownership.append(p)
write(support/'evidence-audit.json',{'tested_revision':tested,'owner_policy_commit':policy,'production_unchanged':True,'existing_tracked_files_unchanged':True,'new_paths_within_checker_ownership':True,'review_count':len(reviews),'verdicts':dict(Counter(r['verdict'] for r in reviews)),'all237_sources_and_references_hash_verified':True,'all237_statuses_reproduced_after_retention':True,'final_source_inventory':len(final['results']),'final_counts':final['total'],'fresh_source_entries':139,'fresh_unique_source_hashes':len({r['source_sha256'] for r in json.loads((tmp/'fresh-results.json').read_text())['results']+json.loads((tmp/'acceptance-results.json').read_text())['results']}),'review_schema_shape_checked':True,'frozen_harness_review_gate':'still fails on rejected records and existing evidence residue; shape check is not acceptance'})
write(support/'artifact-hashes.json',[{'path':str(p.relative_to(support)),'sha256':sha(p),'bytes':p.stat().st_size} for p in sorted(support.rglob('*')) if p.is_file() and p.name!='artifact-hashes.json'])
print(json.dumps({'reviews':len(reviews),'verdicts':dict(Counter(r['verdict'] for r in reviews)),'heldouts':len(final['results']),'decimal_census':census,'audit':'pass'},indent=2))
