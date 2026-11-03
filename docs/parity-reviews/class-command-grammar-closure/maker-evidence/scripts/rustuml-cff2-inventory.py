# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Real CLI inventory using the unchanged strict comparator and frozen corpus."""
from pathlib import Path
import subprocess,os,json,hashlib,concurrent.futures,sys
phase=sys.argv[1]; root=Path.cwd();out=Path('/tmp/rustuml-cff2')/(phase+'833');out.mkdir(exist_ok=True)
binary='/tmp/rustuml-cff2-before-rustuml' if phase=='before' else '/tmp/rustuml-cff-target/release/rustuml'
old=json.loads(Path('docs/parity-reviews/class-command-execution-ownership/maker-evidence/perturbation-results.json').read_text())
rows=old['cases'];assert len(rows)==833
invalid={r['source'] for r in rows if r.get('after')=='invalid_reference' or r.get('before')=='invalid_reference'}
# The previous inventory stores frozen invalid-reference status; this is not a
# new marker definition. Preserve all inputs and comparator outputs regardless.
print('row keys',rows[0].keys(), 'invalid frozen',len(invalid),flush=True)
env=os.environ.copy();env.update(PLANTUML_STDLIB='/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib',RUSTUML_DEBUG='date=1774210426000,tz=AEDT+1100')
def run(row):
 source=Path(row['source']);dest=out/source.relative_to('test-diagrams/perturbations');dest.parent.mkdir(parents=True,exist_ok=True)
 r=subprocess.run([binary,'-tsvg',str(source)],capture_output=True,env=env,timeout=60)
 dest.with_suffix('.rust.svg').write_bytes(r.stdout);dest.with_suffix('.stderr.txt').write_bytes(r.stderr)
 a=subprocess.run([binary,'--ast',str(source)],capture_output=True,env=env,timeout=60);dest.with_suffix('.ast.txt').write_bytes(a.stdout+a.stderr)
 c=subprocess.run(['/tmp/rustuml-cff-check-svg',str(source.with_suffix('.svg')),str(dest.with_suffix('.rust.svg'))],capture_output=True,timeout=60)
 dest.with_suffix('.diff.txt').write_bytes(c.stdout+c.stderr)
 return {'source':str(source),'cohort':row['cohort'],'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'reference_sha256':hashlib.sha256(source.with_suffix('.svg').read_bytes()).hexdigest(),'svg_sha256':hashlib.sha256(r.stdout).hexdigest(),'ast_sha256':hashlib.sha256(a.stdout).hexdigest(),'cli_exit':r.returncode,'ast_exit':a.returncode,'comparison_exit':c.returncode,'factory':a.stdout.decode().split('(',1)[0].strip(),'status':'invalid_reference' if str(source) in invalid else ('process_error' if r.returncode else 'pass' if not c.returncode else 'mismatch'),'cli_stderr':r.stderr.decode(),'comparison':(c.stdout+c.stderr).decode()}
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool: results=list(pool.map(run,rows))
(out/'results.json').write_text(json.dumps(results,indent=2)+'\n')
from collections import Counter
print(Counter(r['status'] for r in results),flush=True)
