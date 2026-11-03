# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Measure the prepared 913 separately, temporarily parking only our new pairs."""
import importlib.util
from pathlib import Path
import shutil
import subprocess

path=Path(__file__).resolve().parent/'check.py'
spec=importlib.util.spec_from_file_location('check',path)
m=importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
park=m.SCRATCH/'own-pairs-during-prepared-replay'
park.mkdir(exist_ok=False)
moved=[]
try:
    for mechanism in m.MECHANISMS:
        source=Path('test-diagrams/perturbations')/mechanism
        destination=park/mechanism
        shutil.move(source,destination)
        moved.append((source,destination))
    env=dict(m.ENV,CARGO_TARGET_DIR=str(m.SCRATCH/'target'),CARGO_BUILD_JOBS='4',RAYON_NUM_THREADS='4')
    result=subprocess.run(['cargo','test','--release','--test','parity_perturbations','--','--nocapture'],env=env,capture_output=True,timeout=300)
    (m.SUPPORT/'prepared-builtin.log').write_bytes(result.stdout+result.stderr)
    (m.SUPPORT/'prepared-builtin.status').write_text(str(result.returncode)+'\n')
finally:
    for source,destination in moved:
        shutil.move(destination,source)
    park.rmdir()
print('prepared built-in exit',result.returncode)
