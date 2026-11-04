# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Read-only Git and source provenance checks, independent of maker counts."""
import hashlib
import json
import re
import subprocess
from pathlib import Path

R = Path('/Users/marcelo/.t3/worktrees/rustuml/t3-6ec78fcf')
S = Path('/Users/marcelo/Library/Caches/rustuml-tim8-checker')
before = '2cf6c7eeebae7354e7a3b7e53b86fb3754883e67'
candidate = 'faf463445591a53f17d25c5c16611f8e626ef93d'
imported = '95c114314f479be301ff9549f4f1d3edf2616a28'
rejection = 'a7b3be50308f93ab2d97d6aee5fb9fbda522b7f2'
account = '02fa49de2396c1445a4ddf1a783c973f883a82f4'
implementation = '2274e808433568aa1d8042bf9e3cb1f1de338b4b'
sha = lambda b: hashlib.sha256(b).hexdigest()
def git(*args):
    return subprocess.check_output(['git', '-C', str(R), *args])
paths = ['docs/parity-reviews', 'test-diagrams/perturbations']
tree = git('ls-tree', '-rz', imported, '--', *paths)
entries = [x.split(b'\t', 1) for x in tree.split(b'\0') if x]
rows = []
proc = subprocess.Popen(['git', '-C', str(R), 'cat-file', '--batch'], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
for metadata, path in entries:
    mode, kind, blob = metadata.split()
    assert kind == b'blob'
    proc.stdin.write(blob + b'\n'); proc.stdin.flush()
    header = proc.stdout.readline().split()
    data = proc.stdout.read(int(header[2])); assert proc.stdout.read(1) == b'\n'
    current = (R / path.decode()).read_bytes()
    rows.append(dict(path=path.decode(), git_blob=blob.decode(), bytes=len(data), sha256=sha(data), unchanged=data == current))
proc.stdin.close(); assert proc.wait() == 0
module = 'crates/rustuml-parser/src/preprocess/mod.rs'
old = git('show', before + ':' + module)
new = git('show', candidate + ':' + module)
marker = b'    #[test]\n    fn filename_without_source_environment_remains_empty()'
old_tail = old[old.index(marker):]
new_tail = new[new.index(marker):]
chronology = []
for a, b in [(before, imported), (imported, account), (account, implementation), (implementation, candidate)]:
    code = subprocess.run(['git', '-C', str(R), 'merge-base', '--is-ancestor', a, b]).returncode
    chronology.append(dict(ancestor=a, descendant=b, exit=code))
    assert code == 0
mechanisms = ['tim-memory-trie-text-lookup', 'tim-duplicate-formal-binding', 'tim-empty-sub-iterator-termination']
accounts = []
for m in mechanisms:
    p = 'docs/parity-reviews/' + m + '/account.json'
    commits = git('log', '--format=%H', candidate, '--', p).decode().splitlines()
    assert commits == [account], commits
    assert git('show', account + ':' + p) == (R / p).read_bytes()
    accounts.append(dict(path=p, commits=commits, sha256=sha((R / p).read_bytes())))
J = Path('/Users/marcelo/work/github.com/plantuml/plantuml')
jrev = '71806a23780b04a5ccde2f8ceb5121edad5eb711'
jpaths = [x['path'] for x in json.loads((R / 'docs/parity-reviews/tim-memory-trie-text-lookup/maker-support/java-source-provenance.json').read_bytes())]
jpaths += ['src/main/java/net/sourceforge/plantuml/tim/' + x for x in ['expression/ReversePolishInterpretor.java', 'EaterFunctionCall.java', 'iterator/CodeIteratorIf.java', 'iterator/CodeIteratorWhile.java', 'iterator/CodeIteratorForeach.java']]
java = []
for p in jpaths:
    data = subprocess.check_output(['git', '-C', str(J), 'show', jrev + ':' + p])
    local = (J / p).read_bytes()
    java.append(dict(path=p, revision=jrev, sha256=sha(data), checkout_matches=data == local))
    assert data == local
result = dict(candidate=candidate, before=before, imported=imported, rejection=rejection,
              historical_files=len(rows), modified_historical=[x for x in rows if not x['unchanged']],
              original_test_blocks=old_tail.count(b'#[test]'), original_test_tail_identical=old_tail == new_tail,
              added_tests=new.count(b'#[test]') - old.count(b'#[test]'), module_sha256=sha(new),
              module_at_account_identical_to_before=git('show', account + ':' + module) == old,
              chronology=chronology, accounts=accounts,
              rejection_to_import_evidence_diff=git('diff', '--name-status', rejection, imported, '--', *paths).decode(),
              implementation_to_candidate_production_diff=git('diff', '--name-status', implementation, candidate, '--', 'crates').decode(),
              implementation_changes=git('show', '--format=fuller', '--stat', implementation).decode(),
              git_status=git('status', '--short').decode(), java_sources=java)
(S / 'integrity.json').write_text(json.dumps(result, indent=2) + '\n')
(S / 'historical-files.json').write_text(json.dumps(rows, indent=2) + '\n')
print(json.dumps({k:v for k,v in result.items() if k not in ['java_sources','implementation_changes','accounts','chronology']}, indent=2))
