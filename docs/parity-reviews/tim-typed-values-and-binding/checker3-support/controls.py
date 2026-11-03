# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Recheck existing sources without rewriting any reference."""
import concurrent.futures,json
from pathlib import Path
import check
old=check.ROOT/'docs/parity-reviews/tim-expression-event-sink/checker2-support'
rows=[dict(source=r['source'],kind='class TIM handoff') for r in json.loads((old/'class-tim-handoff.json').read_text())['results']]
rows += [dict(source=r['source'],kind='raw reader regression control') for r in json.loads((check.ROOT/'docs/parity-reviews/tim-corrective-execution-maker/maker-results.json').read_text())['raw_reader_files']]
def control(row):
 p=Path(row['source']);out=check.S/'reference'/p.parent.name/p.stem
 args=['--cwd',check.REMOTE,'--timezone','GMT+11:00']
 r=check.remote(args+['--','-pipe','-tsvg'],(check.ROOT/p).read_bytes());check.save(out/'java.svg',r.stdout);check.save(out/'java.stderr',r.stderr)
 q=check.remote(args+['--probe',check.REMOTE+'/SourceProbe.java','--classpath',check.REMOTE,'--',check.REMOTE+'/'+str(p),check.REMOTE]);check.save(out/'java.source',q.stdout);check.save(out/'source.stderr',q.stderr)
 c=check.run([check.S/'probe','compare',check.ROOT/p.with_suffix('.svg'),out/'java.svg']);check.save(out/'reference-comparison.txt',c.stdout+c.stderr)
 result=check.measure(row,'after',True);result.update(java_exit=r.returncode,java_source_exit=q.returncode,remote_vs_preserved_match=c.returncode==0)
 return result
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:results=list(pool.map(control,rows))
check.dump(check.SUP/'regression-controls.json',results);print(check.summary(results));print('remote references',sum(r['remote_vs_preserved_match'] for r in results))
