# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Package already captured evidence; never run Java, change gates, or claim acceptance.

Run from the isolated repository: python package_evidence.py PRIVATE_CACHE.
"""
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
DEST = ROOT / 'docs/parity-reviews/sequence-lifecycle-decoration-packet/maker-results'
PREVIOUS = ROOT / 'docs/parity-reviews/sequence-rejection-repair'

def digest(data):
    return hashlib.sha256(data).hexdigest()

def read(path):
    data = path.read_bytes()
    return json.loads(gzip.decompress(data) if path.suffix == '.gz' else data)

def write(name, value):
    data = (json.dumps(value, indent=2) + '\n').encode()
    path = DEST / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(gzip.compress(data, mtime=0) if path.suffix == '.gz' else data)

def revision(short):
    return subprocess.check_output(['git', 'rev-parse', short], text=True).strip()

def stats(rows):
    return dict(entries=len(rows), matches=sum(r['cli_comparison']['exit'] == 0 for r in rows),
                valid_mismatches=sum(r['cli_comparison']['exit'] == 1 for r in rows),
                invalid_references=sum(r['cli_comparison']['exit'] == 2 for r in rows),
                valid_cli_errors=[r['source'] for r in rows if r['cli_comparison']['exit'] != 2 and r['cli']['exit'] != 0],
                invalid_cli_rejected=sum(r['cli_comparison']['exit'] == 2 and r['cli']['exit'] == 1 for r in rows),
                invalid_cli_accepted=sum(r['cli_comparison']['exit'] == 2 and r['cli']['exit'] == 0 for r in rows),
                cli_exit_counts=dict(collections.Counter(r['cli']['exit'] for r in rows)),
                comparator_exit_counts={mode: dict(collections.Counter(r[mode]['exit'] for r in rows)) for mode in ['cli_comparison', 'product_library', 'assisted_library']},
                cli_equals_product_library=sum(r['cli_equals_library'] for r in rows),
                cli_panics=[r['source'] for r in rows if 'panicked at' in r['cli']['stderr']])

after = read(CACHE / 'final1828/measurements.json')
before_old = read(PREVIOUS / 'checker-support/replay1728.json.gz')
before_checker = read(PREVIOUS / 'checker-support/fresh.json.gz')
before_new = read(CACHE / 'before28/measurements.json') + read(CACHE / 'note4-before/measurements.json')
before = before_old + before_checker + before_new
assert len(before) == len(after) == 1828
by_before = {r['source']: r for r in before}
assert len(by_before) == 1828
changes = []
for row in after:
    old = by_before[row['source']]
    for key in ['source_sha256', 'golden_sha256']:
        assert old[key] == row[key], (row['source'], key)
    for key, hash_key in [('source', 'source_sha256'), ('golden', 'golden_sha256')]:
        assert digest((ROOT / row[key]).read_bytes()) == row[hash_key]
    changes.append(dict(source=row['source'], source_sha256=row['source_sha256'], golden_sha256=row['golden_sha256'],
                        before_comparator=old['cli_comparison']['exit'], after_comparator=row['cli_comparison']['exit'],
                        before_cli=old['cli']['exit'], after_cli=row['cli']['exit']))
losses = [r for r in changes if r['before_comparator'] == 0 and r['after_comparator'] != 0]
gains = [r for r in changes if r['before_comparator'] == 1 and r['after_comparator'] == 0]
losses_by_mode = {mode: [r['source'] for r in after if by_before[r['source']][mode]['exit'] == 0 and r[mode]['exit'] != 0] for mode in ['cli_comparison', 'product_library', 'assisted_library']}
assert not any(losses_by_mode.values())
negative_corrections = [r for r in changes if r['before_comparator'] == 2 and r['before_cli'] == 0 and r['after_cli'] == 1]
assert not losses and not stats(after)['valid_cli_errors']
binaries = read(CACHE / 'final-binaries.json')
assert binaries['revision'] == revision('41e2ce88')
for path, sha in binaries['source_files'].items():
    assert digest((ROOT / path).read_bytes()) == sha
for name, sha in binaries['binaries'].items():
    assert digest((CACHE / 'bin' / name).read_bytes()) == sha
baseline_bin = Path('/Users/marcelo/Library/Caches/rustuml-sequence-repair/bin')
expected_before = {'rustuml':'eb6f4b5e35c28d3cf3899e1c97c08683411f615e89bb461ddca8ab05ffd73184',
                   'diff_one':'b9accff9c5036985205417f39a106d27db84cb89824a80957878e1384b88246b',
                   'compare_cli':'cb92187de32da6d8f659c8add252eb663b88934bffed28ce6e3e9fa0463b1425'}
for name, sha in expected_before.items():
    assert digest((baseline_bin / name).read_bytes()) == sha
write('binaries.json', dict(after=binaries, before=dict(revision=revision('0fb4e14b'), binaries=expected_before, provenance='Prior maker binaries.json authenticates these preserved binaries; new32 before replay used them.')))
write('all-inputs.json', read(CACHE / 'all1828.json'))
write('measurements.json.gz', after)
write('before-new32.json.gz', before_new)
write('before-after.json.gz', changes)
write('remaining-failures.json.gz', [r for r in after if r['cli_comparison']['exit'] != 0])
write('negative-input-status.json', [dict(r, golden_status='invalid reference, never a positive SVG pass') for r in changes if r['after_comparator'] == 2])
write('checks.json', read(CACHE / 'final-checks.json'))
cohort_provenance = read(PREVIOUS / 'checker-support/cohort-provenance.json')
write('cohort-overlap.json', dict(previous_checker_provenance=str((PREVIOUS / 'checker-support/cohort-provenance.json').relative_to(ROOT)),
                               previous_checker_provenance_sha256=digest((PREVIOUS / 'checker-support/cohort-provenance.json').read_bytes()),
                               old_maker_entries=1728, fresh_checker_entries=68, new_maker_entries=32, total=1828,
                               unique_source_paths=len({r['source'] for r in after}), unique_source_bytes=len({r['source_sha256'] for r in after}),
                               duplicate_entries_by_source_bytes=len(after)-len({r['source_sha256'] for r in after}),
                               parent_entries=1449, old_maker_parent_overlap=1123, parent_only_not_replayed=326,
                               note='Do not sum these overlapping cohorts. Parent owns combined integration; no unrelated growth replay or class/TIM production import.'))
main_log = (CACHE / 'final-logs/main-tiers-and-guards.log').read_text()
product = re.search(r'golden_no_oracle: (\d+)/(\d+)', main_log)
assisted = re.search(r'golden_pairs: (\d+) total, (\d+) passed, (\d+) failed, (\d+) skipped', main_log)
assert product and assisted
failure_section = main_log[main_log.index('held-out no-oracle perturbations diverged'):]
builtin_failures = set(re.findall(r'^  (test-diagrams/perturbations/[^\n]+?\.puml):', failure_section, re.M))
standing = [r for r in after if r['source'].startswith('test-diagrams/perturbations/')]
main_current = {}
for name in ['no_oracle_failure_names.txt', 'golden_failure_names.txt']:
    previous = gzip.decompress((PREVIOUS / 'maker-results/logs' / (name+'.gz')).read_bytes())
    current = (ROOT / 'test-diagrams' / name).read_bytes()
    main_current[name] = dict(before_sha256=digest(previous), after_sha256=digest(current),
                             newly_failing=sorted(set(current.decode().splitlines())-set(previous.decode().splitlines())),
                             recovered=sorted(set(previous.decode().splitlines())-set(current.decode().splitlines())))
    (DEST / ('logs/'+name+'.gz')).parent.mkdir(parents=True, exist_ok=True)
    (DEST / ('logs/'+name+'.gz')).write_bytes(gzip.compress(current, mtime=0))
assert all(not x['newly_failing'] for x in main_current.values())
summary = dict(kind='maker measurements; not an acceptance record', implementation=revision('41e2ce88'), baseline=revision('0fb4e14b'),
               imported_independent_evidence=dict(original=revision('7ab58bbc'), local=revision('9af2148f')),
               totals_before=stats(before), totals_after=stats(after), lost_passes=losses, lost_passes_by_mode=losses_by_mode, recoveries=gains,
               negative_semantic_corrections=negative_corrections,
               cohorts={'old1728':dict(before=stats(before_old),after=stats(after[:1728])),
                        'checker68':dict(before=stats(before_checker),after=stats(after[1728:1796])),
                        'new32':dict(before=stats(before_new),after=stats(after[1796:]))},
               main_library_tiers=dict(product_pass=int(product[1]),eligible=int(product[2]), assisted_pass=int(assisted[2]), assisted_fail=int(assisted[3]),
                                       skipped=int(assisted[4]), sequence_pass=1411, sequence_total=1411, failures_vs_prior=main_current,
                                       label='Library tests, not 11251 CLI invocations. Both tiers remain red on frozen ratchet/known failures; no baseline edits.'),
               units=dict(parser=567,renderer=783,ascii_included=True,cli_pipe=13),
               builtin_perturbations=dict(entries=len(standing),matches=len(standing)-len(builtin_failures),failures=len(builtin_failures),
                                         isolated_root=stats(standing),cwd_difference='Existing include control passes at repo cwd, fails at Cargo package cwd (T14.6.2).'),
               guards=dict(passed=4,total=6,failed=['decimal_literal_ratchet','parity_review_records_are_complete']),
               format_owned='pass',format_workspace='pre-existing class formatting failure',
               clippy='Parser/component dead code blocks combined check; renderer-only has 22 retained findings. No new warnings.',
               local_java_launched=False, reference_runtime='Remote headless pinned Monkey Man; exact JAR, OpenJDK21.0.5/Graphviz12.2.0 differ from origin; scoped calibration only.')
write('summary.json', summary)
account_map = [
    ('sequence-command-event-identity','ea313c62',['e6a4d41b']),
    ('sequence-teoz-life-event-scheduling','ea313c62',['19074587']),
    ('sequence-self-cross-decoration','ea313c62',['a8d717c']),
    ('sequence-legacy-destroy-component','bc18300d',['19074587']),
    ('sequence-legacy-message-note-envelope','bc18300d',['07aa1611','41e2ce88']),
    ('sequence-legacy-message-note-alignment','76972e36',['41e2ce88']),
]
write('implementation-map.json', dict(accounts=[dict(mechanism=m,account_commit=revision(a),implementation_commits=[revision(x) for x in impl]) for m,a,impl in account_map],
                                      source_refinements=[revision(x) for x in ['42ec7126','09eec925','14228d60']],
                                      files=binaries['source_files'],ascii_scope='Exactly two test Message constructors gain parallel:false; no ASCII behavior change.'))
fresh_failures = [r for r in after[1728:1796] if r['cli_comparison']['exit'] == 1]
font_paths = [r['source'] for r in after if 'checker_font_control_' in r['source']]
dependencies = [dict(mechanism=cohort,status='Unresolved, deferred next packet; not waived',sources=[r['source'] for r in fresh_failures if r['cohort']==cohort]) for cohort in sorted({r['cohort'] for r in fresh_failures})]
dependencies += [dict(mechanism='Logical fonts and shared family serialization',target='T14.11',status='Unowned unresolved dependency',sources=font_paths,details='SansSerif family quoting and Serif metrics/family remain failed. No host font import or compensating geometry.'),
                 dict(mechanism='Shared entity sequence stereotypes',status='Independent downstream parser dependency; no event-model cause identified',provenance='dependencies/shared-entity/provenance.json',details='Preserved exact bare_quoted and order_stereo_entity source/reference pairs from fc698b9b; no class/shared parser changes.'),
                 dict(mechanism='Other retained corpus mismatches',status='All 542 prior valid mismatches remain active',inventory='remaining-failures.json.gz'),
                 dict(mechanism='Remaining invalid input acceptance',status='12 invalid references still accepted by CLI; not positive parity',inventory='negative-input-status.json',details='Includes two spaced right-border negatives. Owned same-message reactivation and unknown return color negatives reject.'),
                 dict(mechanism='Builtin include cwd',target='T14.6.2',status='Separate dependency',details='Repository-root is the declared CLI comparison base. Cargo package cwd retains one additional include failure.'),
                 dict(mechanism='Standing gates',status='Not green and not waived',details='Frozen ratchets, prior strict mismatches, review completeness pending independent review, decimal guard, pre-existing class format/parser dead code/22 renderer Clippy findings.')]
write('dependencies.json',dependencies)
# Every intermediate CLI output is retained; the final run is distinct and exact-revision bound.
with tarfile.open(DEST / 'cli-captures.tar.gz', 'w:gz') as archive:
    for dirname in ['identity68','lifecycle92','lifecycle96','replay1824','before28','note4-before','note4-after','final1828']:
        for path in sorted((CACHE/dirname).iterdir()):
            if path.is_file(): archive.add(path,arcname=f'{dirname}/{path.name}')
    archive.add(CACHE/'destroy-final.svg',arcname='focused/destroy-final.svg')
for directory in [CACHE, CACHE/'final-logs', CACHE/'revision19074587-final-logs']:
    for path in sorted(directory.glob('*.log')):
        suffix = path.relative_to(CACHE)
        dest = DEST / 'logs' / (str(suffix)+'.gz'); dest.parent.mkdir(parents=True,exist_ok=True)
        dest.write_bytes(gzip.compress(path.read_bytes(),mtime=0))
for name in ['alignment-eight.json','revision19074587-final-checks.json','revision19074587-final-binaries.json']:
    write('diagnostics/'+name,read(CACHE/name))
(DEST/'final_checks.py').write_text('# Copyright 2026 Marcelo Cantos\n# SPDX-License-Identifier: Apache-2.0\n' + (CACHE/'final_checks.py').read_text())
# Verify that historical tracked evidence and frozen contracts were not edited.
modified = subprocess.check_output(['git','diff','--diff-filter=M','--name-only','9af2148f','HEAD'],text=True).splitlines()
assert set(modified)==set(binaries['source_files']),modified
write('protected-evidence.json',dict(base=revision('9af2148f'),head=revision('41e2ce88'),modified_existing_paths=modified,
                                    invariant='Only the four authorized production/test files modified since imported review evidence. All existing accounts/reviews/fixtures/harness/baselines unchanged.',
                                    comparator_source_sha256=digest((ROOT/'docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs').read_bytes()),
                                    measure_source_sha256=digest((ROOT/'docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py').read_bytes())))
commands={'replay':['python3','docs/parity-reviews/sequence-dependency-closure/checker-support/measure.py',str(CACHE/'all1828.json'),str(CACHE/'bin'),str(CACHE/'final1828')],
          'cwd':str(ROOT),'reference':'No JVM invocation in this task environment. Parent-relayed exact manifests/bytes, raw streams/status and remote runtime metadata retained.',
          'reference_requests':['reference-request.json','reverse-cross-request.json','legacy-note-request.json'],
          'new_before':['before28/measurements.json','note4-before/measurements.json'],
          'before_old':['docs/parity-reviews/sequence-rejection-repair/checker-support/replay1728.json.gz','docs/parity-reviews/sequence-rejection-repair/checker-support/fresh.json.gz']}
write('commands.json',commands)
print(json.dumps({k:summary[k] for k in ['implementation','totals_after','units','guards']},indent=2))
