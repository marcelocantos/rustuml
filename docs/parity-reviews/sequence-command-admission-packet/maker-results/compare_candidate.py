# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import concurrent.futures,json,subprocess,collections,sys
from pathlib import Path
C=Path(__file__).parent;revision=sys.argv[1];out=C/revision;rows=json.loads((out/'results.json').read_text());before={x['source']:x for x in json.loads((C/'before3533/measurements.json').read_text())};gold={x['source']:x for x in json.loads((C/'before-golden-cli/results.json').read_text())}
def cmp(g,r):
 p=subprocess.run([str(C/'before-bin/compare_cli'),g,str(r)],capture_output=True);return dict(exit=p.returncode,stdout=p.stdout.decode(),stderr=p.stderr.decode())
def one(r):
 result=cmp(r['golden'],out/r['stdout_file']);old=before.get(r['source']);b=old['cli_comparison'] if old else cmp(r['golden'],C/'before-golden-cli'/gold[r['source']]['stdout_file']) if r['source'] in gold else None
 return dict(source=r['source'],before=b,after=result)
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool: results=list(pool.map(one,rows))
(out/'comparisons.json').write_text(json.dumps(results,indent=2)+'\n');lost=[x for x in results if x['before'] and x['before']['exit']==0 and x['after']['exit']!=0];print('statuses',collections.Counter(x['after']['exit'] for x in results),'lost',len(lost));print('\n'.join(x['source'] for x in lost))
