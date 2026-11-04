# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Validate newly registered evidence without rerunning product gates."""
import hashlib
import json
import subprocess
import tarfile
from pathlib import Path

R = Path('/Users/marcelo/.t3/worktrees/rustuml/t3-6ec78fcf')
S = Path('/Users/marcelo/Library/Caches/rustuml-tim8-checker')
D = R / 'docs/parity-reviews/tim-memory-trie-text-lookup/checker8-support'
sha = lambda data: hashlib.sha256(data).hexdigest()
read = lambda path: json.loads(path.read_bytes())
manifest = read(D / 'archive-integrity.json')
assert sha((D / 'measurements.tar.gz').read_bytes()) == manifest['sha256']
expected = {row['path']: row for row in read(D / 'archive-files.json')}
seen = set()
with tarfile.open(D / 'measurements.tar.gz', 'r:gz') as archive:
    for item in archive:
        assert item.isfile() and item.name not in seen
        seen.add(item.name)
        data = archive.extractfile(item).read()
        row = expected[item.name]
        assert len(data) == row['bytes'] and sha(data) == row['sha256'], item.name
assert seen == set(expected) and len(seen) == manifest['files']
history = read(D / 'historical-files.json')
for row in history:
    assert sha((R / row['path']).read_bytes()) == row['sha256'], row['path']
assert subprocess.check_output(['git', '-C', str(R), 'diff', '--name-only', 'HEAD']) == b''
assert subprocess.check_output(['git', '-C', str(R), 'rev-parse', 'HEAD']).decode().strip() == 'faf463445591a53f17d25c5c16611f8e626ef93d'
fresh = read(D / 'fresh-results.json')
names = []
for row in fresh:
    source = Path(row['row']['source'])
    name = source.stem
    dest = R / 'test-diagrams/perturbations' / row['row']['mechanism'] / 'checker8'
    assert (dest / source.name).read_bytes() == source.read_bytes()
    assert (dest / source.with_suffix('.svg').name).read_bytes() == (D / 'cases' / name / 'java.svg.stdout').read_bytes()
    names.append(name)
assert len(names) == len(set(names)) == 20
for resource in (S / 'new').glob('*.iuml'):
    dest = R / 'test-diagrams/perturbations/tim-empty-sub-iterator-termination/checker8' / resource.name
    assert dest.read_bytes() == resource.read_bytes()
reviews = []
for mechanism in sorted({row['row']['mechanism'] for row in fresh}):
    path = R / 'docs/parity-reviews' / mechanism / 'review.json'
    review = read(path)
    assert review['schema_version'] == 1 and review['verdict'] == 'rejected'
    assert review['mechanism'] == mechanism
    assert review['reviewer']['task_id'] and review['reviewer']['model']
    assert review['commands'] and review['findings']
    for row in review['perturbations']:
        assert len(row['axes']) >= 2
        assert Path(row['source']).with_suffix('.svg') == Path(row['golden'])
        assert (R / row['source']).is_file() and (R / row['golden']).is_file()
        assert row['result'] in ('pass', 'fail')
    reviews.append(dict(path=str(path.relative_to(R)), cases=len(review['perturbations']), verdict=review['verdict']))
result = dict(candidate='faf463445591a53f17d25c5c16611f8e626ef93d',
              archive_files_verified=len(seen), archive_sha256=manifest['sha256'],
              historical_files_unchanged=len(history), fresh_pairs_exact=len(names),
              include_resources_exact=3, tracked_modifications=[], reviews=reviews,
              scope='Evidence registration only; no broad product gate repeated.')
(D / 'registration-validation.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result, indent=2))
