# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections,gzip,hashlib,json,subprocess,tarfile
from pathlib import Path
root=Path.cwd(); cache=Path('/Users/marcelo/Library/Caches/rustuml-sequence-review-b9c6cf04');p=root/'docs/parity-reviews/sequence-lifecycle-decoration-packet/maker-results'
def sha(b):return hashlib.sha256(b).hexdigest()
def git(*args):return subprocess.check_output(['git',*args],cwd=root,text=True).strip()
def gz(path):return json.loads(gzip.decompress(path.read_bytes()))
m=json.loads((p/'implementation-map.json').read_text()); records=[]
for a in m['accounts']:
 path=f'docs/parity-reviews/{a["mechanism"]}/account.json'
 records.append(dict(**a,account_sha256=sha((root/path).read_bytes()),account_creation=git('log','--diff-filter=A','--format=%H','--',path),preceded_all=all(subprocess.run(['git','merge-base','--is-ancestor',a['account_commit'],i],cwd=root).returncode==0 for i in a['implementation_commits']),implementation_messages=[git('show','-s','--format=%B',i) for i in a['implementation_commits']]))
rows=gz(p/'measurements.json.gz'); before=gz(p/'before-after.json.gz'); corpus=json.loads((p/'all-inputs.json').read_text());problems=[]
assert [r['source'] for r in rows]==[r['source'] for r in corpus]
for r in rows:
 for k in ('source','golden'):
  if sha((root/r[k]).read_bytes())!=r[k+'_sha256']:problems.append((r['source'],k))
with tarfile.open(p/'cli-captures.tar.gz') as t:
 names=set(t.getnames())
 for r in rows:
  name='final1828/'+r['cli']['svg']
  if name not in names:problems.append(('missing capture',name))
  elif sha(t.extractfile(name).read())!=r['cli']['svg_sha256']:problems.append(('capture hash',name))
summary=dict(candidate=git('rev-parse','HEAD'),before='0fb4e14b7227eb768d268128d42fa5f6d5a5b1af',java_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd='/Users/marcelo/work/github.com/plantuml/plantuml',text=True).strip(),accounts=records,source_refinement_commits=[dict(commit=i,message=git('show','-s','--format=%B',i)) for i in m['source_refinements']],production_file_hashes={f:dict(expected=h,actual=sha((root/f).read_bytes())) for f,h in m['files'].items()},ascii_diff=git('diff','0fb4e14b','5358314a','--','crates/rustuml-render/src/ascii.rs'),candidate_vs_production_crates_diff=git('diff','41e2ce88','5358314a','--','crates'),corpus_entries=len(rows),unique_source_byte_strings=len({r['source_sha256'] for r in rows}),source_and_capture_hash_problems=problems,archived_before_counts=dict(collections.Counter(r['before_comparator'] for r in before)),archived_after_counts=dict(collections.Counter(r['after_comparator'] for r in before)),archived_gains=[r['source'] for r in before if r['before_comparator']!=0 and r['after_comparator']==0],archived_lost=[r['source'] for r in before if r['before_comparator']==0 and r['after_comparator']!=0],archived_corrected_negatives=[r['source'] for r in before if r['before_comparator']==2 and r['before_cli']==0 and r['after_cli']!=0])
(cache/'audit.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k not in ('accounts','source_refinement_commits','ascii_diff','archived_gains','archived_corrected_negatives')},indent=2));print('gains',len(summary['archived_gains']),'corrected negatives',len(summary['archived_corrected_negatives']))
