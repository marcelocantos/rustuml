# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import subprocess,sys
from pathlib import Path
S=Path(__file__).parent
cohorts=['retained','maker18','sprites50','checker38','fresh15','fresh26','new15','delimiters4','focused2','checker30','supplement8','fresh12','checker28','fresh16','golden-regressions']
for phase in ['before','after']:
 subprocess.run([sys.executable,str(S/'run_replay.py'),phase,*cohorts],check=True)
