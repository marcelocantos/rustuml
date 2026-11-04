# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Package measured sequence maker evidence without changing frozen records."""
import collections
import gzip
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tarfile

ROOT=Path.cwd()
CACHE=Path(__file__).parent
DEST=ROOT/'docs/parity-reviews/sequence-decoration-height-packet/maker-results'
DEST.mkdir(parents=True,exist_ok=True)
FILES=['crates/rustuml-parser/src/diagram/sequence.rs','crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs','crates/rustuml-render/src/ascii.rs']
MODES=['cli_comparison','product_library','assisted_library']
def read(p):return json.loads(p.read_text())
def sha(data):return hashlib.sha256(data).hexdigest()
def rev(r):return subprocess.check_output(['git','rev-parse',r],text=True).strip()
def write(name,obj):
 p=DEST/name;p.parent.mkdir(parents=True,exist_ok=True);b=(json.dumps(obj,indent=2)+'\n').encode();p.write_bytes(gzip.compress(b,mtime=0) if name.endswith('.gz') else b)
def stats(rows):
 return dict(entries=len(rows),cli_comparator_exits=dict(collections.Counter(r['cli_comparison']['exit'] for r in rows)),matches=sum(r['cli_comparison']['exit']==0 for r in rows),valid_mismatches=sum(r['cli_comparison']['exit']==1 for r in rows),invalid_references=sum(r['cli_comparison']['exit']==2 for r in rows),invalid_rejected=sum(r['cli_comparison']['exit']==2 and r['cli']['exit']==1 for r in rows),invalid_accepted=sum(r['cli_comparison']['exit']==2 and r['cli']['exit']==0 for r in rows),valid_cli_errors=[r['source'] for r in rows if r['cli_comparison']['exit']!=2 and r['cli']['exit']!=0],cli_panics=[r['source'] for r in rows if 'panicked at' in r['cli']['stderr']],library_render_panics=[r['source'] for r in rows if r['library_render_exit']==101],library_comparison_panics={m:[r['source'] for r in rows if r[m]['exit']==101] for m in ['product_library','assisted_library']},mode_exits={m:dict(collections.Counter(r[m]['exit'] for r in rows)) for m in MODES},cli_equals_library=sum(r['cli_equals_library'] for r in rows))
# Keep raw measurement paths intact; provide a separate relocatable manifest.
before=read(CACHE/'before1962/measurements.json')+read(CACHE/'underline-before/measurements.json')
after=read(CACHE/'final71/measurements.json')
assert len(before)==len(after)==1966
changes=[]
for old,new in zip(before,after):
 assert old['source']==new['source']
 for key,hashkey in [('source','source_sha256'),('golden','golden_sha256')]:assert old[hashkey]==new[hashkey]==sha((ROOT/new[key]).read_bytes())
 changes.append(dict(source=new['source'],golden=new['golden'],source_sha256=new['source_sha256'],golden_sha256=new['golden_sha256'],before_comparator=old['cli_comparison']['exit'],after_comparator=new['cli_comparison']['exit'],before_cli=old['cli']['exit'],after_cli=new['cli']['exit']))
losses={m:[n['source'] for o,n in zip(before,after) if o[m]['exit']==0 and n[m]['exit']!=0] for m in MODES}
assert not any(losses.values()) and not stats(after)['valid_cli_errors'] and not stats(after)['cli_panics']
write('before-measurements.json.gz',before);write('measurements.json.gz',after);write('before-after.json.gz',changes)
write('remaining-failures.json.gz',[r for r in after if r['cli_comparison']['exit']!=0])
write('negative-status.json',[r for r in changes if r['after_comparator']==2])
manifest=[]
for row in after:
 entry={k:row[k] for k in ['source','golden','cohort']}
 for key in ['source','golden']:
  source=Path(row[key])
  if source.is_absolute():
   target=DEST/'inputs'/source.name;target.parent.mkdir(exist_ok=True);shutil.copyfile(source,target);entry[key]=str(target.relative_to(ROOT))
 manifest.append(entry)
write('all-inputs.json',manifest)
# Raw returned bytes retain their remote environments/argv/stdout/stderr hashes.
for name in ['reference-results.json.gz','reference-transport.json','reference-manifest.json','reference-classification.json','reference-provenance.json','underline-reference-results.json.gz','underline-reference-transport.json','underline-manifest.json','underline-classification.json']:
 shutil.copyfile(CACHE/name,DEST/name)
shutil.copyfile(CACHE/'dependencies.json', DEST/'dependencies.json')
for name in ['generate_controls.py','prepare.py','focused.py','import_underline.py','final_checks.py','package_evidence.py']:
 shutil.copyfile(CACHE/name,DEST/name)
# All intermediate render and model outputs, including regression and negatives.
with tarfile.open(DEST/'captures.tar.gz','w:gz') as archive:
 for directory in ['before1962','candidate1962','final1962','final-rule1962','final71','initial61','second61','third61','fourth61','model-diagnostics','model-final','underline-before','underline-final','underline-rule-final','underline-corrected','main-regression','validation350e776a','validation026e5eeb']:
  for p in sorted((CACHE/directory).rglob('*')):
   if p.is_file():archive.add(p,arcname=str(p.relative_to(CACHE)))
 for p in sorted(CACHE.glob('*.log')):archive.add(p,arcname=p.name)
for p in sorted((CACHE/'final-logs').glob('*.log')):
 target=DEST/'logs'/(p.name+'.gz');target.parent.mkdir(exist_ok=True);target.write_bytes(gzip.compress(p.read_bytes(),mtime=0))
checks=read(CACHE/'final-checks.json');write('checks.json',checks)
bindings=read(CACHE/'final-binaries.json')
for path,digest in bindings['source_files'].items():assert sha((ROOT/path).read_bytes())==digest
for name,digest in bindings['binaries'].items():assert sha((CACHE/'bin'/name).read_bytes())==digest
old_bins=read(ROOT/'docs/parity-reviews/sequence-corrective-b9/maker-results/binaries.json')['after']
for name,digest in old_bins['binaries'].items():assert sha((CACHE/'before-bin'/name).read_bytes())==digest
write('binaries.json',dict(before=dict(baseline=rev('efc2e922'),authenticated_by='../../sequence-corrective-b9/maker-results/binaries.json',original=old_bins),after=bindings,committed_production=rev('HEAD'),binding='Build metadata records its actual checkout revision, possibly dirty; source-file SHA256 values match committed production exactly.'))
log=(CACHE/'final-logs/main-tiers-and-guards.log').read_text()
product=re.search(r'golden_no_oracle: (\d+)/(\d+)',log);assisted=re.search(r'golden_pairs: (\d+) total, (\d+) passed, (\d+) failed, (\d+) skipped',log)
assert product and assisted
main_changes={}
for name in ['no_oracle_failure_names.txt','golden_failure_names.txt']:
 old=gzip.decompress((ROOT/'docs/parity-reviews/sequence-corrective-b9/maker-results/logs'/(name+'.gz')).read_bytes());new=(ROOT/'test-diagrams'/name).read_bytes()
 main_changes[name]=dict(newly_failing=sorted(set(new.decode().splitlines())-set(old.decode().splitlines())),recovered=sorted(set(old.decode().splitlines())-set(new.decode().splitlines())),sha256=sha(new))
 (DEST/'logs'/(name+'.gz')).write_bytes(gzip.compress(new,mtime=0))
assert all(not r['newly_failing'] for r in main_changes.values())
units=re.findall(r'test result: ok\. (\d+) passed;', (CACHE/'final-logs/units.log').read_text())
cli=re.findall(r'test result: ok\. (\d+) passed;', (CACHE/'final-logs/cli.log').read_text())
registered=len(list((ROOT/'test-diagrams/perturbations').rglob('*.puml')))
section=log[log.index('held-out no-oracle perturbations diverged'):]
builtin_failures=set(re.findall(r'^  (test-diagrams/perturbations/[^\n]+?\.puml):',section,re.M))
summary=dict(kind='Maker evidence only; no acceptance claim',baseline=rev('efc2e922'),implementation=rev('HEAD'),before=stats(before),after=stats(after),gains=[r for r in changes if r['before_comparator']==1 and r['after_comparator']==0],losses_by_mode=losses,
 cohorts={name:dict(before=stats(before[a:b]),after=stats(after[a:b])) for name,a,b in [('preserved1901',0,1901),('checker37',1901,1938),('maker24',1938,1962),('underline4',1962,1966)]},
 main_library_tiers=dict(product_pass=int(product[1]),eligible=int(product[2]),assisted_pass=int(assisted[2]),assisted_failed=int(assisted[3]),skipped=int(assisted[4]),sequence='1411/1411',changes=main_changes,label='Library tiers, not 11251 CLI processes; frozen test ratchets remain red'),units=dict(parser=int(units[0]),renderer=int(units[1]),ascii_included=True,cli_pipe=int(cli[0])),guards='4/6',
 builtin_perturbations=dict(registered=registered,failures=len(builtin_failures),matches=registered-len(builtin_failures),cwd=str(ROOT/'crates/rustuml-oracle'),note='Unchanged built-in suite. Package cwd differs from isolated CLI repo cwd on an existing theme case (T14.6.2).'),
 validation_retry='350e776a introduced one main creole-family regression. All intermediate logs/output are preserved. Accounts54fb67fc and6e9c711d precede the source-derived flow/title corrections; four intermediate fresh mismatches are also preserved. The subsequent plain-participant Teoz creation height audit changed its consumer to the same resolved shadow; validation026e5eeb is also preserved. Final repetition is justified by changed production, not an attempt to erase failed runs.')
write('summary.json',summary)
write('cohort-overlap.json',dict(entries=len(after),unique_source_paths=len({r['source'] for r in after}),unique_source_bytes=len({r['source_sha256'] for r in after}),duplicate_entries_by_source_bytes=len(after)-len({r['source_sha256'] for r in after}),retained1901=1901,fresh_checker37=37,maker28=28,checker_registered39_includes_two_prior_maker_copies=True,parent1449_overlap=1123,parent_only_not_replayed=326,overlap_provenance='../../sequence-corrective-b9/maker-results/cohort-overlap.json',note='Frozen parent1449 and maker1938 overlap; do not sum. Every retained entry is measured, including duplicates. No unrelated parent growth imported.'))
imported=subprocess.check_output(['git','diff-tree','--no-commit-id','--name-only','-r','fdf1154e'],text=True).splitlines()
assert not subprocess.check_output(['git','diff','--name-only','fdf1154e','HEAD','--',*imported],text=True).strip()
modified=subprocess.check_output(['git','diff','--diff-filter=M','--name-only','eaedee41','HEAD'],text=True).splitlines();assert set(modified)==set(FILES)
write('protected-evidence.json',dict(imported_original=rev('fdf1154e'),local_import=rev('eaedee41'),imported_paths=len(imported),immutable_import_verified=True,modified_existing_paths=modified,comparator_sha256=sha((ROOT/'docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs').read_bytes()),measure_sha256=sha((ROOT/'docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py').read_bytes()),ascii_diff='Exactly two target_circle: false additions beside source_circle; no other edits.'))
write('implementation-map.json',dict(accounts=[dict(mechanism=m,account_commit=rev(a),implementation_commit=rev(i)) for m,a,i in [('sequence-target-circle-decoration','0b329582','350e776a'),('sequence-ordinary-note-child-allocation','0b329582','350e776a'),('sequence-component-style-consumption','0b329582','350e776a'),('sequence-message-text-component-height','54fb67fc','026e5eeb'),('sequence-note-rule-title-height','6e9c711d','026e5eeb'),('sequence-component-style-consumption creation consumer','0b329582','71b465a9')]],files=FILES,full_tested_production=rev('HEAD'),note='All historical accepted/bounded observations stay tied to their exact tested revisions. This packet requires fresh independent review.'))
print(json.dumps(dict(before=summary['before'],after=summary['after'],main=summary['main_library_tiers'],builtin=summary['builtin_perturbations']),indent=2))
