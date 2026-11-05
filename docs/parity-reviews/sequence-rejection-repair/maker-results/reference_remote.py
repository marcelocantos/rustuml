# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Capture new raw references on Monkey Man only; never overwrite captures."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

REMOTE = '/Users/marcelo/rustuml-reference/scratch/worker.gKRm76Fl'
COMMAND = ['ssh', '-T', '-o', 'BatchMode=yes', 'marcelo@192.168.1.112',
           f'/Users/marcelo/rustuml-reference/bin/reference --cwd {REMOTE} --timezone GMT+11:00 -- -pipe -tsvg']
ENV = dict(os.environ)
ENV['JAVA_TOOL_OPTIONS'] = (ENV.get('JAVA_TOOL_OPTIONS', '') + ' -Djava.awt.headless=true').strip()
for directory in sys.argv[1:]:
    for source in sorted(Path(directory).glob('*.puml')):
        if source.with_suffix('.svg').exists():
            continue
        result = subprocess.run(COMMAND, input=source.read_bytes(), capture_output=True,
                                env=ENV, timeout=90)
        source.with_suffix('.svg').write_bytes(result.stdout)
        source.with_suffix('.stderr').write_bytes(result.stderr)
        record = dict(command=COMMAND, exit=result.returncode,
                      source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
                      svg_sha256=hashlib.sha256(result.stdout).hexdigest(),
                      java='Homebrew OpenJDK 21.0.5', graphviz='12.2.0',
                      jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',
                      java_source_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',
                      host='Monkey Man 192.168.1.112', timezone='GMT+11:00',
                      headless='forced by reference wrapper', local_java_launched=False,
                      environment_limit='Seven calibration pairs agree; no universal equivalence claimed')
        source.with_suffix('.provenance.json').write_text(json.dumps(record, indent=2) + '\n')
        print(source, result.returncode, len(result.stdout), flush=True)
