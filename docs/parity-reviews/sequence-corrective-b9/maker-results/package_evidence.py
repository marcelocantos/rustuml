# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Package preserved local measurements; does not launch Java or grant acceptance."""
import collections
import gzip
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile

ROOT = Path.cwd()
CACHE = Path(sys.argv[1]).resolve()
DEST = ROOT / 'docs/parity-reviews/sequence-corrective-b9/maker-results'
PREVIOUS = ROOT / 'docs/parity-reviews/sequence-lifecycle-decoration-packet/maker-results'
MODES = ['cli_comparison', 'product_library', 'assisted_library']

def sha(data):
    return hashlib.sha256(data).hexdigest()

def read(path):
    body = path.read_bytes()
    return json.loads(gzip.decompress(body) if path.suffix == '.gz' else body)

def write(name, value):
    data = (json.dumps(value, indent=2) + '\n').encode()
    path = DEST / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(gzip.compress(data, mtime=0) if path.suffix == '.gz' else data)

def rev(value):
    return subprocess.check_output(['git', 'rev-parse', value], text=True).strip()

def stats(rows):
    return dict(entries=len(rows), matches=sum(r['cli_comparison']['exit'] == 0 for r in rows),
                valid_mismatches=sum(r['cli_comparison']['exit'] == 1 for r in rows),
                invalid_references=sum(r['cli_comparison']['exit'] == 2 for r in rows),
                invalid_rejected=sum(r['cli_comparison']['exit'] == 2 and r['cli']['exit'] == 1 for r in rows),
                invalid_accepted=sum(r['cli_comparison']['exit'] == 2 and r['cli']['exit'] == 0 for r in rows),
                valid_cli_errors=[r['source'] for r in rows if r['cli_comparison']['exit'] != 2 and r['cli']['exit'] != 0],
                panics=[r['source'] for r in rows if 'panicked at' in r['cli']['stderr']],
                cli_exit_counts=dict(collections.Counter(r['cli']['exit'] for r in rows)),
                mode_exit_counts={m: dict(collections.Counter(r[m]['exit'] for r in rows)) for m in MODES},
                cli_equals_product_library=sum(r['cli_equals_library'] for r in rows))

before = read(CACHE / 'before1897/measurements.json') + read(CACHE / 'shadow4-before/measurements.json')
after = read(CACHE / 'final1897/measurements.json') + read(CACHE / 'shadow4-final/measurements.json')
assert len(before) == len(after) == 1901
assert len({r['source'] for r in after}) == len(after)
changes = []
for old, new in zip(before, after):
    assert old['source'] == new['source']
    for key, hash_key in [('source', 'source_sha256'), ('golden', 'golden_sha256')]:
        assert old[hash_key] == new[hash_key] == sha((ROOT / new[key]).read_bytes())
    changes.append(dict(source=new['source'], golden=new['golden'], source_sha256=new['source_sha256'], golden_sha256=new['golden_sha256'],
                        before_comparator=old['cli_comparison']['exit'], after_comparator=new['cli_comparison']['exit'],
                        before_cli=old['cli']['exit'], after_cli=new['cli']['exit']))
losses = {m: [n['source'] for o, n in zip(before, after) if o[m]['exit'] == 0 and n[m]['exit'] != 0] for m in MODES}
gains = [r for r in changes if r['before_comparator'] == 1 and r['after_comparator'] == 0]
assert not any(losses.values()) and not stats(after)['valid_cli_errors'] and not stats(after)['panics']
assert stats(after)['matches'] == 1299 and len(gains) == 40
binaries = read(CACHE / 'final-binaries.json')
assert binaries['revision'] == rev('1b29d8c7')
for path, digest in binaries['source_files'].items():
    assert sha((ROOT / path).read_bytes()) == digest
for name, digest in binaries['binaries'].items():
    assert sha((CACHE / 'bin' / name).read_bytes()) == digest
old_bins = read(PREVIOUS / 'binaries.json')['after']
for name, digest in old_bins['binaries'].items():
    assert sha((CACHE / 'before-bin' / name).read_bytes()) == digest
write('binaries.json', dict(after=binaries, before=dict(revision=rev('5358314a'), production_revision=old_bins['revision'],
      binaries=old_bins['binaries'], provenance='Preserved baseline binaries authenticated by prior maker binaries.json; copied into this private before-bin. No new before build or separate checkout.'),
      source_binding='Final checks and CLI replay bound to exact committed 1b29d8c7 source hashes; no later production change.'))
write('all-inputs.json', read(CACHE / 'all1897.json') + read(CACHE / 'shadow4.json'))
write('all1897-inputs.json', read(CACHE / 'all1897.json'))
write('measurements.json.gz', after)
write('before-measurements.json.gz', before)
write('before-after.json.gz', changes)
write('remaining-failures.json.gz', [r for r in after if r['cli_comparison']['exit'] != 0])
write('negative-input-status.json', [dict(r, classification='Invalid Java reference; CLI rejection is not an SVG parity pass') for r in changes if r['after_comparator'] == 2])
write('checks.json', read(CACHE / 'final-checks.json'))

main_log = (CACHE / 'final-logs/main-tiers-and-guards.log').read_text()
product = re.search(r'golden_no_oracle: (\d+)/(\d+)', main_log)
assisted = re.search(r'golden_pairs: (\d+) total, (\d+) passed, (\d+) failed, (\d+) skipped', main_log)
assert product and assisted
main_changes = {}
for name in ['no_oracle_failure_names.txt', 'golden_failure_names.txt']:
    old = gzip.decompress((PREVIOUS / 'logs' / (name + '.gz')).read_bytes())
    new = (ROOT / 'test-diagrams' / name).read_bytes()
    main_changes[name] = dict(before_sha256=sha(old), after_sha256=sha(new), newly_failing=sorted(set(new.decode().splitlines())-set(old.decode().splitlines())), recovered=sorted(set(old.decode().splitlines())-set(new.decode().splitlines())))
    p = DEST / 'logs' / (name + '.gz'); p.parent.mkdir(exist_ok=True); p.write_bytes(gzip.compress(new, mtime=0))
assert all(not r['newly_failing'] for r in main_changes.values())
assert 'sequence          1411/1411' in main_log
units = re.findall(r'test result: ok\. (\d+) passed;', (CACHE / 'final-logs/units.log').read_text())
cli_units = re.findall(r'test result: ok\. (\d+) passed;', (CACHE / 'final-logs/cli.log').read_text())
assert units == ['568', '787'] and cli_units == ['13']
# The built-in suite runs at Cargo package cwd, while isolated CLI runs at repo cwd.
section = main_log[main_log.index('held-out no-oracle perturbations diverged'):]
builtin_failures = set(re.findall(r'^  (test-diagrams/perturbations/[^\n]+?\.puml):', section, re.M))
standing = [r for r in after if r['source'].startswith('test-diagrams/perturbations/')]
root_failures = {r['source'] for r in standing if r['cli_comparison']['exit'] != 0}
cohorts = {label: dict(before=stats(before[start:stop]), after=stats(after[start:stop])) for label, start, stop in [('old1728', 0, 1728), ('checker68', 1728, 1796), ('prior-maker32', 1796, 1828), ('retained1828', 0, 1828), ('checker49', 1828, 1877), ('new-maker20', 1877, 1897), ('shadow-value4', 1897, 1901), ('new-maker24', 1877, 1901)]}
summary = dict(kind='Maker measurements, not independent acceptance', baseline=rev('5358314a'), implementation=rev('1b29d8c7'),
      imported_evidence=dict(original=rev('68c8b95e'), local=rev('37642c05')),
      totals_before=stats(before), totals_after=stats(after), gains=gains, lost_passes_by_mode=losses,
      cohorts=cohorts, preserved_accepted_axes='All 8 self-cross and 4 legacy-destroy checker controls still match. Bounded acceptance remains attached only to full tested 5358314a; no extracted or candidate acceptance.',
      main_library_tiers=dict(product_pass=int(product[1]), eligible=int(product[2]), assisted_pass=int(assisted[2]), assisted_fail=int(assisted[3]), skipped=int(assisted[4]), sequence_pass=1411, sequence_total=1411, failures_vs_baseline=main_changes, label='Library tests, not 11251 CLI processes. Frozen ratchets and existing failures remain red.'),
      units=dict(parser=568, renderer=787, ascii_included=True, cli_pipe=13),
      builtin_perturbations=dict(entries=len(standing), matches=len(standing)-len(builtin_failures), failures=len(builtin_failures), isolated_repo_cwd=stats(standing), additional_package_cwd_failures=sorted(builtin_failures-root_failures)),
      guards=dict(passed=4, total=6, failed=['decimal_literal_ratchet','parity_review_records_are_complete']),
      format_owned='pass', format_workspace='Existing class formatting failure', clippy='Combined check blocked by existing parser/component dead code; renderer-only retains 20 existing findings (previously 22), no newly introduced finding.',
      intermediate_failures='First focused69: 53 matches,12 valid mismatches,4 invalid. First full main:11178 product/10494 assisted, three introduced activation-border regressions, subsequently repaired; all logs and before/intermediate/final CLI captures retained.',
      local_java_launched=False, network_reference_launches='None; parent relay returned exact remote captures',
      reference_runtime='Pinned Monkey Man JAR; remote OpenJDK21.0.5/Graphviz12.2.0 differ from origin. Seven-case transport calibration was scoped, not global equivalence.')
write('summary.json', summary)
write('cohort-overlap.json', dict(retained_entries=1828, new_checker_entries=49, new_maker_entries=24, total=1901,
      unique_source_paths=len({r['source'] for r in after}), unique_source_bytes=len({r['source_sha256'] for r in after}), duplicate_entries_by_source_bytes=len(after)-len({r['source_sha256'] for r in after}),
      parent_entries=1449, old1728_parent_overlap=1123, parent_only_not_replayed=326, provenance='Prior maker cohort-overlap.json and checker cohort-provenance.json; no unrelated parent growth imported or replayed. Do not sum overlapping cohorts. No duplicates removed.'))
accounts = [('sequence-note-display-normalization','8e1440fb',['c5349576']),('sequence-effective-arrow-color','8e1440fb',['ecd996a3','8d668d3f']),('sequence-legacy-raw-life-variations','8e1440fb',['f77e328c']),('sequence-teoz-parallel-tile-ordinates','8e1440fb',['7e074830','8d668d3f']),('sequence-effective-note-activation-style','8e1440fb',['8d668d3f']),('sequence-note-textblock-dimensions','2dab3648',['8d668d3f']),('sequence-note-shadow-value-materialization','2d17c23b',['1b29d8c7'])]
write('implementation-map.json', dict(accounts=[dict(mechanism=m,account_commit=rev(a),implementation_commits=[rev(i) for i in impl]) for m,a,impl in accounts], files=binaries['source_files'], tested_revision=rev('1b29d8c7'), note='Intermediate local commits are not independently accepted; verification and requested fresh review concern the full candidate.'))
dependencies = read(PREVIOUS / 'dependencies.json')
for item in dependencies:
    if item['mechanism'] == 'Standing gates':
        item['details'] = 'Frozen ratchets, existing main failures, rejected reviews and new unreviewed accounts, decimal guard, existing class format/parser dead code/20 renderer Clippy findings.'
    if item['mechanism'] == 'Shared entity sequence stereotypes':
        item['provenance'] = '../../sequence-lifecycle-decoration-packet/maker-results/dependencies/shared-entity/provenance.json'
dependencies += [dict(mechanism='Themed self-arrow/note preferred allocation', status='Two new valid mismatches retained; no acceptance or geometry waiver',
      sources=[r['source'] for r in after[1877:] if r['cli_comparison']['exit']==1],
      details='Cerulean legacy self-note is 10px left and the canvas 10px short; Teoz counterpart has participant/note/canvas allocation differences. Existing constraint_global_padding discards theme padding in self_arrow_by_event while Java AbstractComponentRoseArrow.getTextBlock -> sequenceDiagramPadder -> MessageSelfArrow.getPreferredWidth -> Step1Abstract.createNoteBox uses the component width. The Teoz counterpart additionally needs CommunicationTileSelf/CommunicationTileNote allocation audit. Activation border colors now agree; do not tune geometry to hide this separate preferred-component dependency.'),
      dict(mechanism='Broader Display/color semantics outside tested correction', status='Source coverage limits, not waived or claimed accepted', details='This packet normalizes inline backslash-n/tab/backslash and preserves raw math/link and block text. Display natural-alignment backslash-r/backslash-l and other Jaws controls are not ported here. HColorAutomagic is accepted by existing grammar but reference HColor.toColor throws unless a caller resolves getAppropriateColor; arrow automatic paint has no captured positive reference in this packet. No grammar narrowing or automatic-color acceptance claim.')]
write('dependencies.json', dependencies)

write('library-invalid-reference-status.json', [dict(source=n['source'], before={m:o[m] for m in MODES}, after={m:n[m] for m in MODES}, classification='Unchanged diff_one panic on an invalid empty Java reference; CLI returns success, retained as negative acceptance, not a new valid-input renderer panic.') for o,n in zip(before,after) if any(n[m]['exit']==101 for m in MODES)])
# Preserve every raw local CLI stream and measurement, including intermediate failures.
with tarfile.open(DEST / 'cli-captures.tar.gz', 'w:gz') as archive:
    for directory in ['before1897','focus-first','focus-second','after1897','final1897','validation-8d668d3f/final1897','main-regression-before','main-regression-intermediate','main-regression-fixed','shadow4-before','shadow4-intermediate','shadow4-final']:
        for path in sorted((CACHE / directory).iterdir()):
            if path.is_file():
                archive.add(path, arcname=f'{directory}/{path.name}')
for directory in [CACHE, CACHE/'final-logs', CACHE/'validation-first', CACHE/'validation-first/final-logs', CACHE/'validation-8d668d3f', CACHE/'validation-8d668d3f/final-logs']:
    for path in sorted(directory.glob('*.log')):
        dest = DEST/'logs'/(str(path.relative_to(CACHE))+'.gz');dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(gzip.compress(path.read_bytes(),mtime=0))
for name in ['final-checks.json','final-binaries.json','no_oracle_failure_names.txt','golden_failure_names.txt']:
    src = CACHE/'validation-first'/name
    dest = DEST/'diagnostics'/'validation-first'/name;dest.parent.mkdir(parents=True, exist_ok=True);shutil.copyfile(src,dest)
for name in ['implementation.patch','color-stage.patch','rawlife-stage.patch','teoz-stage.patch','combo-regression.diff']:
    p = DEST/'diagnostics'/(name+'.gz'); p.parent.mkdir(parents=True, exist_ok=True);p.write_bytes(gzip.compress((CACHE/name).read_bytes(),mtime=0))
for name in ['generate_controls.py','import_references.py','import_shadow.py','final_checks.py','package_evidence.py']:
    shutil.copyfile(CACHE/name, DEST/name)
write('commands.json', dict(cwd=str(ROOT), replay_helper='docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py',
      replays=[dict(manifest='all1897-inputs.json',bin_dir=str(CACHE/'before-bin'),output=str(CACHE/'before1897')),dict(manifest='all1897-inputs.json',bin_dir=str(CACHE/'bin'),output=str(CACHE/'final1897'))],
      reference_request='reference-request.json', reference_capture='reference-results.json.gz', shadow_reference_request='shadow-reference-request.json', shadow_reference_capture='shadow-reference-results.json.gz', shadow_reference_return_gzip_sha256='92f4fa57a19633895c66ce8135f3530625489db2714371c23ad5482f3fbca15d', shadow_reference_input_transport_sha256='8a08dd1baa4c0f8dc3afc0293acff72b2beaf07eee9c86c836b1cc075c03b04d', reference_transport='/Users/marcelo/Library/Caches/rustuml-campaign-relay/sequence20-transport.json',
      reference_input_transport_sha256='f3a9ffa25d1e9e30b872ed9738e586fb51d4251b44f10ff346a9e77acf9fd2a1', reference_return_gzip_sha256='084b60166689371184df41d99f39e7340efc8cb42c3363f3c12ed92e0f80a5ef',
      note='Final rerun justified by three discovered main regressions. Earlier1897 outputs are compared with final1897; source-value conversion audit justified one final rerun. No further full repeats. No local JVM, SSH, network change or reference rerun.'))
modified = subprocess.check_output(['git','diff','--diff-filter=M','--name-only','37642c05','HEAD'],text=True).splitlines()
assert set(modified)==set(binaries['source_files'])
imported_paths = subprocess.check_output(['git','diff-tree','--no-commit-id','--name-only','-r','68c8b95e'],text=True).splitlines()
assert not subprocess.check_output(['git','diff','--name-only','68c8b95e','HEAD','--',*imported_paths],text=True).strip()
write('protected-evidence.json', dict(base=rev('37642c05'), head=rev('1b29d8c7'), modified_existing_paths=modified,
      imported_review_paths_verified=len(imported_paths), imported_evidence_preserved_byte_for_byte=True,
      comparator_source_sha256=sha((ROOT/'docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs').read_bytes()),
      measure_source_sha256=sha((ROOT/'docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py').read_bytes()),
      invariant='All existing accounts/reviews/fixtures/harness/comparator/baselines/targets unchanged. Production changes restricted to sequence parser and renderer.'))
print(json.dumps(dict(totals=summary['totals_after'], gains=len(gains), main=summary['main_library_tiers'], overlap=read(DEST/'cohort-overlap.json')),indent=2))
