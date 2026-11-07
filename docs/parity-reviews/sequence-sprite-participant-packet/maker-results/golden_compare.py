# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures,json,subprocess,sys,collections
from pathlib import Path
C=Path(__file__).parent;revision=sys.argv[1];D=C/(revision+'-golden-cli');rows=json.loads((D/'results.json').read_text())
def one(r):
 golden=Path(r['source']).with_suffix('.svg');p=subprocess.run([str(C/'before-bin/compare_cli'),str(golden),str(D/r['stdout_file'])],capture_output=True);return dict(source=r['source'],comparison=dict(exit=p.returncode,stdout=p.stdout.decode(),stderr=p.stderr.decode()))
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:results=list(pool.map(one,rows))
(D/'comparisons.json').write_text(json.dumps(results,indent=2)+'\n');print(revision,collections.Counter(r['comparison']['exit'] for r in results))
