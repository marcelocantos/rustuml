# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Deliberately package verified private sequence captures; never runs Java."""
import collections, gzip, hashlib, json, re, shutil, subprocess, tarfile
from pathlib import Path
ROOT = Path.cwd()
CACHE = Path(__file__).parent
DEST = ROOT / 'docs/parity-reviews/sequence-composed-components-packet/maker-results'
BASE = '092cf5f68464625c8b3771e069b42f44f43fc0c9'
FILES = ['crates/rustuml-parser/src/parse/mod.rs','crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-parser/src/diagram/sequence.rs','crates/rustuml-render/src/sequence.rs','crates/rustuml-render/src/ascii.rs']
MODES = ['cli_comparison', 'product_library', 'assisted_library']
def read(p): return json.loads(p.read_text())
def sha(b): return hashlib.sha256(b).hexdigest()
def git(*args): return subprocess.check_output(['git', *args], text=True).strip()
def write(n, value):
    p=DEST/n; p.parent.mkdir(parents=True,exist_ok=True)
    data=(json.dumps(value,indent=2)+'\n').encode()
    p.write_bytes(gzip.compress(data,mtime=0) if n.endswith('.gz') else data)
def stats(rows):
    return dict(entries=len(rows),matches=sum(x['cli_comparison']['exit']==0 for x in rows),valid_mismatches=sum(x['cli_comparison']['exit']==1 for x in rows),invalid_references=sum(x['cli_comparison']['exit']==2 for x in rows),invalid_cli_exits=dict(collections.Counter(x['cli']['exit'] for x in rows if x['cli_comparison']['exit']==2)),cli_exits=dict(collections.Counter(x['cli']['exit'] for x in rows)),valid_cli_errors=[x['source'] for x in rows if x['cli_comparison']['exit']!=2 and x['cli']['exit']!=0],cli_panics=[x['source'] for x in rows if 'panicked at' in x['cli']['stderr']],library_render_panics=[x['source'] for x in rows if x['library_render_exit']==101],library_diagnostic_panics={m:[x['source'] for x in rows if x[m]['exit']==101] for m in MODES[1:]},mode_exits={m:dict(collections.Counter(x[m]['exit'] for x in rows)) for m in MODES},cli_equals_library=sum(x['cli_equals_library'] for x in rows))
DEST.mkdir(parents=True,exist_ok=True)
before=read(CACHE/'before1997/measurements.json')+read(CACHE/'before22/measurements.json')+read(CACHE/'before2/measurements.json')
after=read(CACHE/'stable2021/measurements.json')
assert len(before)==len(after)==2021
changes=[]
for old,new in zip(before,after):
    assert old['source']==new['source']
    for key in ['source','golden']:
        assert old[key+'_sha256']==new[key+'_sha256']==sha((ROOT/new[key]).read_bytes())
    changes.append(dict(source=new['source'],golden=new['golden'],source_sha256=new['source_sha256'],golden_sha256=new['golden_sha256'],before_comparator=old['cli_comparison']['exit'],after_comparator=new['cli_comparison']['exit'],before_cli=old['cli']['exit'],after_cli=new['cli']['exit'],before_svg_sha256=old['cli']['svg_sha256'],after_svg_sha256=new['cli']['svg_sha256']))
losses={m:[n['source'] for o,n in zip(before,after) if o[m]['exit']==0 and n[m]['exit']!=0] for m in MODES}
assert not any(losses.values()) and not stats(after)['valid_cli_errors'] and not stats(after)['cli_panics']
write('before-measurements.json.gz',before);write('measurements.json.gz',after);write('before-after.json.gz',changes)
write('remaining-failures.json.gz',[r for r in after if r['cli_comparison']['exit']!=0]);write('negative-status.json',[r for r in changes if r['after_comparator']==2])
manifest=[]
for row in after:
    item={k:row[k] for k in ['source','golden','cohort']}
    for key in ['source','golden']:
        p=Path(row[key])
        if p.is_absolute():
            target=DEST/'inputs'/p.name;target.parent.mkdir(exist_ok=True);shutil.copy2(p,target);item[key]=str(target.relative_to(ROOT))
    manifest.append(item)
write('all-inputs.json',manifest)
for p in (CACHE/'reference-inputs').glob('*.stderr'):shutil.copy2(p,DEST/'inputs'/p.name)
# Preserve original request, relay bytes, classifications and source bindings.
for name in ['reference-results.json.gz','reference-transport.json','reference-manifest.json','reference-classification.json','parent-relay-validation.json','transfer-provenance.json','reference-input-bundle.json.gz','actor-reference-results.json.gz','actor-reference-transport.json','actor-manifest.json','actor-classification.json','actor-parent-validation.json','actor-transfer-provenance.json','actor-input-bundle.json.gz','local-reference-validation.json','before-binding.json','final-binaries.json','stable-binaries.json','first-broad-delta.json','queue-filter-binaries.json','head-alignment-binaries.json']:
    shutil.copy2(CACHE/name,DEST/name)
for p in CACHE.glob('*.py'):shutil.copy2(p,DEST/p.name)
# Every intermediate measurement/SVG and log is retained; compiled binaries are
# private and represented by hashes, not committed as build artefacts.
excluded_dirs={p.name for p in CACHE.iterdir() if p.is_dir() and (p.name.endswith('-bin') or p.name=='bin')}
with tarfile.open(DEST/'captures.tar.gz','w:gz') as archive:
    for p in sorted(CACHE.rglob('*')):
        rel=p.relative_to(CACHE)
        if p.is_file() and rel.parts[0] not in excluded_dirs:
            archive.add(p,arcname=str(rel))
for directory in ['final-logs','stable-logs']:
    for p in (CACHE/directory).glob('*.log'):
        q=DEST/'logs'/directory/(p.name+'.gz');q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(gzip.compress(p.read_bytes(),mtime=0))
checks=read(CACHE/'stable-checks.json');write('checks.json',checks)
binding=read(CACHE/'stable-binaries.json')
assert binding['revision']==git('rev-parse','HEAD')
for p,h in binding['source_files'].items():assert sha((ROOT/p).read_bytes())==h
for n,h in binding['binaries'].items():assert sha((CACHE/'stable-bin'/n).read_bytes())==h
for n,h in read(CACHE/'before-binding.json')['binaries'].items():assert sha((CACHE/'before-bin'/n).read_bytes())==h
write('binaries.json',dict(before=read(CACHE/'before-binding.json'),before_baseline=BASE,after=binding,binding='Final build from clean committed production; source files and all binary hashes verified. Before production is byte-identical to baseline 092cf5f; original binding names 71b465a9.'))
log=(CACHE/'stable-logs/main-tiers-and-guards.log').read_text()
product=re.search(r'golden_no_oracle: (\d+)/(\d+)',log);assisted=re.search(r'golden_pairs: (\d+) total, (\d+) passed, (\d+) failed, (\d+) skipped',log)
assert product and assisted
main_changes={}
for name in ['no_oracle_failure_names.txt','golden_failure_names.txt']:
    old=gzip.decompress((ROOT/'docs/parity-reviews/sequence-decoration-height-packet/maker-results/logs'/(name+'.gz')).read_bytes());new=(ROOT/'test-diagrams'/name).read_bytes()
    main_changes[name]=dict(newly_failing=sorted(set(new.decode().splitlines())-set(old.decode().splitlines())),recovered=sorted(set(old.decode().splitlines())-set(new.decode().splitlines())),sha256=sha(new))
    p=DEST/'logs'/(name+'.gz');p.parent.mkdir(exist_ok=True);p.write_bytes(gzip.compress(new,mtime=0))
assert all(not x['newly_failing'] for x in main_changes.values())
units=re.findall(r'test result: ok\. (\d+) passed;', (CACHE/'stable-logs/units.log').read_text());cli=re.findall(r'test result: ok\. (\d+) passed;', (CACHE/'stable-logs/cli.log').read_text())
registered=len(list((ROOT/'test-diagrams/perturbations').rglob('*.puml')))
section=log[log.index('held-out no-oracle perturbations diverged'):];builtin_failures=set(re.findall(r'^  (test-diagrams/perturbations/[^\n]+?\.puml):',section,re.M))
summary=dict(kind='Maker evidence only; independent review required',baseline=BASE,implementation=git('rev-parse','HEAD'),before=stats(before),after=stats(after),gains=[r for r in changes if r['before_comparator']==1 and r['after_comparator']==0],losses_by_mode=losses,cohorts={name:dict(before=stats(before[a:b]),after=stats(after[a:b])) for name,a,b in [('preserved1966',0,1966),('checker31',1966,1997),('maker22',1997,2019),('actor2',2019,2021)]},main_library_tiers=dict(product_pass=int(product[1]),eligible=int(product[2]),assisted_pass=int(assisted[2]),assisted_failed=int(assisted[3]),skipped=int(assisted[4]),sequence='1411/1411',changes=main_changes,label='Library tiers, not 11251 real CLI processes'),units=dict(parser=int(units[0]),renderer=int(units[1]),ascii_included=True,cli_pipe=int(cli[0])),guards='4/6',builtin_perturbations=dict(registered=registered,failures=len(builtin_failures),matches=registered-len(builtin_failures),cwd=str(ROOT/'crates/rustuml-oracle'),note='Standing suite remains red; package cwd differs from isolated CLI repo cwd on existing T14.6.2 include case. No exclusions.'),validation_repetition='The c921945a first broad run regressed four queue goldens (11177/11251, sequence1407/1411, assisted10493). New account7ab48cc4 precedes queue preferred-size repair. Scoped actor controls still failed; account00845306 precedes alignment repair. Code changed, justifying final complete replay. First failures and transient compile/setup errors remain in captures.tar.gz.')
write('summary.json',summary)
cohorts={}
for name,a,b in [('preserved1966',0,1966),('checker31',1966,1997),('maker22',1997,2019),('actor2',2019,2021)]:
    rows=after[a:b];cohorts[name]=dict(entries=len(rows),unique_sources=len({r['source_sha256'] for r in rows}),overlap_previous=len({r['source_sha256'] for r in rows}&{r['source_sha256'] for r in after[:a]}))
write('cohort-overlap.json',dict(entries=len(after),unique_source_paths=len({r['source'] for r in after}),unique_source_bytes=len({r['source_sha256'] for r in after}),duplicate_entries_by_source_bytes=len(after)-len({r['source_sha256'] for r in after}),cohorts=cohorts,parent1449_overlap=1123,parent_only_not_replayed=326,parent_overlap_provenance='../../sequence-decoration-height-packet/maker-results/cohort-overlap.json',note='Parent1449 is overlapping, not additive. No unrelated growth imported. Every retained duplicate entry is measured.'))
imported=git('diff-tree','--no-commit-id','--name-only','-r','607ad3e7').splitlines();assert not set(imported) & set(git('diff','--name-only','607ad3e7','HEAD').splitlines())
modified=git('diff','--diff-filter=M','--name-only','9d81627b','HEAD').splitlines();assert set(modified)==set(FILES)
old_evidence=git('ls-tree','-r','--name-only',BASE,'--','docs/parity-reviews','test-diagrams/perturbations').splitlines();assert not set(old_evidence) & set(git('diff','--name-only',BASE,'HEAD').splitlines())
write('protected-evidence.json',dict(original_import=git('rev-parse','607ad3e7'),local_import=git('rev-parse','9d81627b'),imported_paths=len(imported),immutable_import_verified=True,prior_evidence_paths=len(old_evidence),immutable_prior_evidence_verified=True,modified_existing_files=modified,comparator_sha256=sha((ROOT/'docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs').read_bytes()),measure_sha256=sha((ROOT/'docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py').read_bytes()),ascii_diff='Exactly two source_dressing: None constructor additions. Renderer untouched; actual ASCII changes are separately preserved.'))
write('implementation-map.json',dict(accounts=[dict(mechanism=m,account_commit=git('rev-parse',a),implementation_commits=[git('rev-parse',i) for i in commits]) for m,a,commits in [('sequence-factory-command-viability','5b45a172',['aeb638a3']),('sequence-endpoint-dressing-composition','5b45a172',['96716b8b']),('sequence-note-body-composition','5b45a172',['689f00cb','c921945a']),('sequence-created-participant-component','5b45a172',['c921945a','bc9ef612']),('sequence-queue-preferred-component','7ab48cc4',['bc9ef612']),('sequence-resolved-head-alignment','00845306',['b4f37ab5'])]],files=FILES,full_tested_production=git('rev-parse','HEAD'),ownership='parse/mod.rs only decorated command viability; ascii.rs exactly two source_dressing:None additions. No shared style/text/font changes.'))
models=read(CACHE/'stable-model-and-ascii.json');write('model-and-ascii.json.gz',models)
write('model-ascii-summary.json',dict(entries=len(models),changed_ascii=[r['source'] for r in models if not r['ascii_equal']],note='Factory ownership changes four ASCII outputs; no claim of ASCII behavior invariance. All raw model and ASCII stdout/stderr/status retained.'))
print(json.dumps(dict(before=summary['before'],after=summary['after'],main=summary['main_library_tiers'],builtin=summary['builtin_perturbations']),indent=2))
