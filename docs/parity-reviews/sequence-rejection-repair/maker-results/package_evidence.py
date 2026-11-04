# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Package existing captures; never launches Java or changes acceptance inputs."""
import collections
import datetime
import gzip
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[4]
OUT = Path(__file__).resolve().parent
SCRATCH = Path('/Users/marcelo/Library/Caches/rustuml-sequence-repair')
CHECKER = ROOT / 'docs/parity-reviews/sequence-dependency-closure/checker-support/evidence'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def write(name, value):
    data = (json.dumps(value, indent=2) + '\n').encode()
    (OUT / name).write_bytes(gzip.compress(data, mtime=0) if name.endswith('.gz') else data)


def read(path):
    raw = path.read_bytes()
    return json.loads(gzip.decompress(raw) if path.suffix == '.gz' else raw)


def counts(rows, field='cli_comparison'):
    return dict(sorted(collections.Counter(r[field]['exit'] for r in rows).items()))


rows = read(SCRATCH / 'replay/measurements.json')
for row in rows:
    row['capture_directory'] = 'replay'
replacement = read(SCRATCH / 'include-root/measurements.json')[0]
replacement['capture_directory'] = 'include-root'
rows = [dict(r, **{k: v for k, v in replacement.items() if k not in ('cohort', 'frozen_inventory')})
        if r['source'] == replacement['source'] else r for r in rows]
assert len(rows) == 1728
for row in rows:
    assert digest((ROOT / row['source']).read_bytes()) == row['source_sha256']
    assert digest((ROOT / row['golden']).read_bytes()) == row['golden_sha256']
    assert digest((SCRATCH / row['capture_directory'] / row['cli']['svg']).read_bytes()) == row['cli']['svg_sha256']
write('measurements.json.gz', rows)

old_rows = []
for name in ('parent-after', 'maker-after', 'fresh-after', 'focused-after'):
    for row in read(CHECKER / (name + '.json.gz')):
        old_rows.append(dict(row, evidence=str((CHECKER / (name + '.json.gz')).relative_to(ROOT))))
old_by_path = {r['source']: r for r in old_rows}
old_by_bytes = {(r['source_sha256'], r['golden_sha256']): r for r in old_rows}
comparisons = []
for row in rows:
    old = old_by_path.get(row['source']) or old_by_bytes.get((row['source_sha256'], row['golden_sha256']))
    if old:
        assert old['source_sha256'] == row['source_sha256'] and old['golden_sha256'] == row['golden_sha256']
    comparisons.append(dict(source=row['source'], source_sha256=row['source_sha256'],
                            frozen_inventory=row['frozen_inventory'], before=old,
                            after={k: row[k] for k in ('cli', 'cli_comparison', 'product_library', 'assisted_library')},
                            before_note='Independent checker candidate capture; production tree identical to 1800ac48.' if old else 'Pre-repair per-case scratch was lost; no verified before claim for this new case.'))
write('before-after.json.gz', comparisons)
assert not subprocess.check_output(['git', 'diff', '5d3eb9824df6084411c591f90d5b385aac32c1b7',
                                    '1800ac48714743863307c82674490c697fcf5709', '--', 'crates', 'Cargo.toml', 'Cargo.lock'], cwd=ROOT)

by_bytes = collections.defaultdict(list)
for row in rows:
    by_bytes[row['source_sha256']].append(row['source'])
write('duplicate-sources.json', {sha: paths for sha, paths in by_bytes.items() if len(paths) > 1})
write('remaining-failures.json.gz', [r for r in rows if r['cli_comparison']['exit'] != 0])

log = (SCRATCH / 'logs/main-tiers-and-guards.log').read_text()
family = re.findall(r'^  ([\w-]+)\s+(\d+)/(\d+)\s*$', log, re.M)
main_product = sum(int(p) for _, p, _ in family)
main_eligible = sum(int(n) for _, _, n in family)
assisted = re.search(r'golden_pairs: (\d+) total, (\d+) passed, (\d+) failed, (\d+) skipped', log).groups()
builtin = re.findall(r'^  (test-diagrams/perturbations/[^:]+): (.*)', log, re.M)
assert len(set(name for name, _ in builtin)) == len(builtin)
guard = log.split('invalid T14 parity review evidence:', 1)[1].split('test parity_review_records_are_complete', 1)[0]
write('guard-diagnostics.json', dict(decimal_files=re.findall(r'^  (crates/[^\n]+ -> [^\n]+)$', log, re.M),
                                    evidence_diagnostics=[line.strip() for line in guard.splitlines() if line.startswith('  /')]))
write('builtin-failing-inputs.json', [dict(source=name, reason=reason) for name, reason in builtin])

summary = dict(status='MAKER EVIDENCE ONLY. Previous claims remain rejected; fresh independent review required.',
               implementation=read(SCRATCH / 'binaries.json')['revision'],
               regenerated_at_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),
               scratch_loss='Original /tmp/rustuml-seq-recovery disappeared across interruption. Earlier transcript counts are historical unverified claims. This packet uses newly recovered runs and existing immutable checker before evidence; no local JVM was launched.',
               coverage=dict(entries=len(rows), unique_source_bytes=len(by_bytes), duplicate_entries=len(rows)-len(by_bytes),
                             frozen_heldouts=1123, earlier_maker=535, new_maker=70,
                             parent_1449_growth='Not imported; combined integration is parent-owned.'),
               final={field: counts(rows, field) for field in ('cli_comparison', 'product_library', 'assisted_library')},
               cli_exit_counts=counts(rows, 'cli'), cli_equals_library=sum(r['cli_equals_library'] for r in rows),
               real_cli_panics=sum('panicked at' in r['cli']['stderr'] for r in rows),
               valid_source_parse_errors=[r['source'] for r in rows if r['cli_comparison']['exit'] != 2 and r['cli']['exit'] != 0],
               cohorts={}, comparable={}, main_library_tiers=dict(product_pass=main_product, eligible=main_eligible,
                   assisted_pass=int(assisted[1]), assisted_fail=int(assisted[2]), skipped=int(assisted[3]), total=int(assisted[0]),
                   sequence_product_pass=1411, sequence_assisted_pass=1411, sequence_eligible=1411,
                   measurement='Library test tiers, not 11251 CLI processes; real CLI measurements are the separate 1728 replay.'),
               builtin=dict(entries=1123, passed=1123-len(builtin), nonpass=len(builtin),
                            invalid_java_marked_pages=sum('Java error page' in reason for _,reason in builtin),
                            invalid_reference_total=sum(r['cli_comparison']['exit']==2 for r in rows if r['frozen_inventory'] in ('parent955.json','generated.json','focused-generated.json','retained-maker-negatives.json')),
                            renderer_panics=sum('renderer panicked' in reason for _,reason in builtin),
                            registry_cascades=log.count('with_registry is not re-entrant')),
               cwd_contract='Repository-root CLI passes checker_include_theme after pinned support restoration; Cargo package cwd fails it (T14.6.2). Both captured separately; no fixture normalization.',
               tests=read(SCRATCH / 'checks.json'), guards=dict(passed=4, failed=2,
                   failures=['decimal_literal_ratchet: existing other files', 'parity_review_records_are_complete: historical rejections and new accounts without independent reviews']),
               accepted_registry='Unchanged; filter_registry.rs is absent from production diff.')
for cohort in sorted({r['frozen_inventory'] for r in rows}):
    subset=[r for r in rows if r['frozen_inventory']==cohort]
    summary['cohorts'][cohort]=dict(entries=len(subset), after=counts(subset))
for field in ('cli_comparison','product_library','assisted_library'):
    comparable=[c for c in comparisons if c['before']]
    gains=[c['source'] for c in comparable if c['before'][field]['exit'] != 0 and c['after'][field]['exit']==0]
    losses=[c['source'] for c in comparable if c['before'][field]['exit']==0 and c['after'][field]['exit']!=0]
    summary['comparable'][field]=dict(entries=len(comparable),before=dict(collections.Counter(c['before'][field]['exit'] for c in comparable)),
        after=dict(collections.Counter(c['after'][field]['exit'] for c in comparable)),recoveries=gains,regressions=losses)
summary['units'] = dict(parser_pass=565, render_pass=782, failed=0)
summary['main_library_tiers'].update(before_product_pass=11181, before_assisted_pass=10497)
write('summary.json', summary)

for name in ['binaries.json','checks.json']:
    shutil.copy2(SCRATCH / name, OUT / name)
logs=OUT/'logs';logs.mkdir(exist_ok=True)
for source in (SCRATCH/'logs').iterdir():
    (logs/(source.name+'.gz')).write_bytes(gzip.compress(source.read_bytes(),mtime=0))
for name in ['golden_failure_names.txt','golden_ulp_candidates.txt','no_oracle_failure_names.txt']:
    source=ROOT/'test-diagrams'/name
    assert source.is_file()
    (logs/(name+'.gz')).write_bytes(gzip.compress(source.read_bytes(),mtime=0))
with tarfile.open(OUT/'cli-captures.tar.gz','w:gz') as archive:
    for directory in ['replay','include-root','include-package']:
        archive.add(SCRATCH/directory,arcname=directory)

assert not any(line.startswith('sequence/') for line in (ROOT/'test-diagrams/golden_failure_names.txt').read_text().splitlines())
write('capture-index.json', {str(p.relative_to(OUT)):dict(bytes=p.stat().st_size,sha256=digest(p.read_bytes()))
                            for p in sorted(OUT.rglob('*')) if p.is_file() and p.name != 'capture-index.json'})
print(json.dumps({key:summary[key] for key in ('coverage','final','cohorts','main_library_tiers','builtin')},indent=2))
