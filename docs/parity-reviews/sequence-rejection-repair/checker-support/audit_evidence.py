# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Recompute reported cohorts and account chronology from preserved objects."""
import collections
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess
ROOT=Path(__file__).resolve().parents[4]
REPORT=Path(__file__).resolve().parent
OUT=Path('/Users/marcelo/Library/Caches/rustuml-sequence-review-d382aba6')
MAKER=ROOT/'docs/parity-reviews/sequence-rejection-repair/maker-results'
def git(*args): return subprocess.check_output(['git',*args],cwd=ROOT)
def sha(data): return hashlib.sha256(data).hexdigest()
def load(p): return json.loads(gzip.decompress(p.read_bytes()) if p.suffix=='.gz' else p.read_bytes())
def counts(rows): return dict(collections.Counter(r['cli_comparison']['exit'] for r in rows))
replay=load(OUT/'replay1728/measurements.json');previous=load(MAKER/'measurements.json.gz')
assert len(replay)==len(previous)==1728
for a,b in zip(replay,previous):
 for key in ['source','golden','source_sha256','golden_sha256']:
  assert a[key]==b[key],key
 for key in ['exit','svg_sha256']:
  assert a['cli'][key]==b['cli'][key],key
 assert a['cli_comparison']['exit']==b['cli_comparison']['exit']
# Check every before observation is exact preserved checker evidence, not maker reconstruction.
preserved=[]
for p in (ROOT/'docs/parity-reviews/sequence-dependency-closure/checker-support/evidence').glob('*.json.gz'):
 d=load(p)
 if isinstance(d,list):
  preserved.extend((str(p.relative_to(ROOT)),r) for r in d if isinstance(r,dict) and 'cli_comparison' in r)
index={}
for path,r in preserved: index.setdefault((r['source_sha256'],r['golden_sha256'],r['cli']['svg_sha256'],r['cli_comparison']['exit']),[]).append(path)
ba=load(MAKER/'before-after.json.gz');matched=[];unknown=[];before=[];after=[]
for r,a in zip(ba,replay):
 assert r['source']==a['source']
 b=r.get('before')
 if b is None: unknown.append(r['source']);continue
 key=(b['source_sha256'],b['golden_sha256'],b['cli']['svg_sha256'],b['cli_comparison']['exit'])
 assert key in index,(r['source'],r.keys())
 assert b['source_sha256']==a['source_sha256'] and b['golden_sha256']==a['golden_sha256']
 matched.append(dict(source=r['source'],evidence=index[key]))
 before.append(b);after.append(a)
recoveries=[a['source'] for b,a in zip(before,after) if b['cli_comparison']['exit']!=0 and a['cli_comparison']['exit']==0]
losses=[a['source'] for b,a in zip(before,after) if b['cli_comparison']['exit']==0 and a['cli_comparison']['exit']!=0]
assert len(before)==1658 and len(unknown)==70
assert len(recoveries)==134 and not losses
# Verify first implementation follows each original account (not merely final head).
chrono=[]
for mechanism,account,impls in re.findall(r'\| (sequence-[^| ]+) \| `([0-9a-f]{40})` \| ([^\n]+) \|',(MAKER/'README.md').read_text()):
 for impl in re.findall(r'`([0-9a-f]{40})`',impls):
  subprocess.run(['git','merge-base','--is-ancestor',account,impl],cwd=ROOT,check=True)
  chrono.append(dict(mechanism=mechanism,account_commit=account,implementation_commit=impl,strict_ancestor=account!=impl))
assert len({x['mechanism'] for x in chrono})==12
# All existing accounts/reviews, baseline files, comparator and parser outside sequence stay immutable.
changed=git('diff','--name-only','1800ac48','0fb4e14b').decode().splitlines()
production=[p for p in changed if p.startswith('crates/')]
assert production==['crates/rustuml-parser/src/diagram/sequence.rs','crates/rustuml-parser/src/parse/sequence.rs','crates/rustuml-render/src/sequence.rs']
assert not git('diff','82914165','0fb4e14b','--','crates','Cargo.toml','Cargo.lock')
# Review import abcf592b must be the original prior review bytes, regardless cherry-pick SHA.
review_index=load(ROOT/'docs/parity-reviews/sequence-dependency-closure/checker-support/evidence/review-index.json')
for p in review_index:
 assert git('show','704b80d0:'+p)==git('show','0fb4e14b:'+p)
parent=load(OUT/'replay1449/measurements.json')
shared=[r for r in parent if (ROOT/r['original_source']).exists()]
additional=[r for r in parent if not (ROOT/r['original_source']).exists()]
summary=dict(candidate='0fb4e14b7227eb768d268128d42fa5f6d5a5b1af',maker_replay=dict(entries=len(replay),counts=counts(replay),unique_source_hashes=len({r['source_sha256'] for r in replay}),all_captures_equal=True),preserved_before=dict(entries=len(before),counts=counts(before),after_counts=counts(after),recoveries=recoveries,losses=losses,no_before=unknown,exact_checker_record_matches=matched),chronology=chrono,production_paths=production,prior_14_verdict_blobs_unchanged=True,parent_collateral=dict(entries=len(parent),counts=counts(parent),shared_entries=len(shared),shared_counts=counts(shared),additional_entries=len(additional),additional_counts=counts(additional),source_hash_overlap_with_maker=len({r['source_sha256'] for r in parent}&{r['source_sha256'] for r in replay}),cwd=str(ROOT),caveat='This is candidate execution on parent fixtures, not parent production or a before/after parent regression claim. No class/TIM candidate imports.'))
(REPORT/'evidence-audit.json').write_text(json.dumps(summary,indent=2)+'\n')
print('Exact maker replay verified; 1658 preserved before records authenticated; 134 recoveries, zero losses; 12 account chronologies; prior 14 review blobs unchanged.')
