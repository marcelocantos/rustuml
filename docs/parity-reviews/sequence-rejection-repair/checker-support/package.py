# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Package durable checker evidence from private cache, never generate references."""
import gzip
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tarfile
ROOT=Path(__file__).resolve().parents[4]
REPORT=Path(__file__).resolve().parent
CACHE=Path('/Users/marcelo/Library/Caches/rustuml-sequence-review-d382aba6')
def sha(b):return hashlib.sha256(b).hexdigest()
def compress(source,dest):dest.write_bytes(gzip.compress(source.read_bytes(),mtime=0))
(REPORT/'logs').mkdir(exist_ok=True)
for p in (CACHE/'logs').glob('*.log'):
 compress(p,REPORT/'logs'/(p.name+'.gz'))
for name in ['checks.json','checks-evidence.json']:
 if (CACHE/name).exists():(REPORT/name).write_bytes((CACHE/name).read_bytes())
for cohort in ['replay1728','replay1449','fresh','include-package']:
 p=CACHE/cohort/'measurements.json'
 if p.exists():compress(p,REPORT/(cohort+'.json.gz'))
# Preserve only genuinely new capture bytes. The replay1728 bytes are proven
# equal to maker cli-captures.tar.gz, so its immutable archive is reused.
with tarfile.open(REPORT/'cli-captures.tar.gz','w:gz') as archive:
 for cohort in ['replay1449','fresh','include-package']:
  p=CACHE/cohort/'measurements.json'
  if not p.exists():continue
  for row in json.loads(p.read_text()):
   if cohort=='replay1449' and (ROOT/row['original_source']).exists():continue
   svg=CACHE/cohort/row['cli']['svg']
   archive.add(svg,arcname=cohort+'/'+svg.name)
# Stream records contain stderr/status; exact .svg bytes retained separately.
files={str(p.relative_to(ROOT)):sha(p.read_bytes()) for p in REPORT.rglob('*') if p.is_file() and p.name!='package-integrity.json'}
binaries={p.name:sha(p.read_bytes()) for p in (CACHE/'final-bin').iterdir()}
report=dict(candidate='0fb4e14b7227eb768d268128d42fa5f6d5a5b1af',production='8291416539aa9c61589e4dd586ad65d3a135966c',build_command='CARGO_TARGET_DIR='+str(CACHE/'target')+' CARGO_BUILD_JOBS=4 cargo build --release -p rustuml -p rustuml-oracle --bin rustuml --example diff_one',comparator_command='rustc --edition=2024 docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs --extern rustuml_oracle='+str(CACHE/'target/release/deps/librustuml_oracle-9463d30e464f1e2a.rlib')+' -L dependency='+str(CACHE/'target/release/deps')+' -o '+str(CACHE/'final-bin/compare_cli'),binaries=binaries,rustc=subprocess.check_output(['rustc','--version'],text=True).strip(),source_trees={p:subprocess.check_output(['git','rev-parse','0fb4e14b:'+p],cwd=ROOT,text=True).strip() for p in ['crates','Cargo.toml','Cargo.lock']},durable_cache=str(CACHE),files=files)
(REPORT/'package-integrity.json').write_text(json.dumps(report,indent=2)+'\n')
print('Packaged',len(files),'evidence files; independent binaries fingerprinted.')
