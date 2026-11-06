# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib, io, json, subprocess, tarfile
from pathlib import Path
R=Path.cwd(); S=Path(__file__).parent
maker=R/'docs/parity-reviews/creole-recursive-character-hiding/maker-support'
archive=maker/'measurements.tar.gz'
assert hashlib.sha256(archive.read_bytes()).hexdigest()=='c8d78419a7f297765ae725f4be2825f66cda1330855f7205c80509a4360e972f'
with tarfile.open(archive) as t:t.extractall(S/'maker',filter='data')
for phase,rev in [('before','a3afe487a66ae26c73e2f1853d6c26fc2d228875'),('after','2608469dace1f680a7e90c0895e44e657f8aaf7f')]:
 dst=S/('source-'+phase);dst.mkdir(exist_ok=True)
 raw=subprocess.check_output(['git','archive',rev,'Cargo.toml','Cargo.lock','crates'])
 with tarfile.open(fileobj=io.BytesIO(raw)) as t:t.extractall(dst,filter='data')
(S/'logs').mkdir(exist_ok=True)
print('Extracted verified maker archive and exact source snapshots')
