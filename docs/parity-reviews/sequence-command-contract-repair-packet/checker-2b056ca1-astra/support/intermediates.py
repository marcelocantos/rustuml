# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,tarfile
from pathlib import Path
C=Path(__file__).parent;M=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-8eddc9af/docs/parity-reviews/sequence-command-contract-repair-packet/maker-results');B=M.parents[1]/'sequence-consumer-contract-packet/maker-results'
sha=lambda b:hashlib.sha256(b).hexdigest();facts=[]
for tier in ['no_oracle_failure_names.txt','golden_failure_names.txt']:
 old=gzip.decompress((B/'logs/final-failure-names'/(tier+'.gz')).read_bytes())
 for phase in ['candidate1','candidate2','final']:
  folder=phase+'-final-failure-names' if phase!='final' else 'final-failure-names'
  new=gzip.decompress((M/'logs'/folder/(tier+'.gz')).read_bytes())
  facts.append(dict(tier=tier,phase=phase,new_losses=sorted(set(new.decode().splitlines())-set(old.decode().splitlines())),recovered=sorted(set(old.decode().splitlines())-set(new.decode().splitlines())),baseline_sha256=sha(old),phase_sha256=sha(new),byte_equal=old==new))
with tarfile.open(M/'captures.tar.gz') as t:
 names=t.getnames(); selected=[n for n in names if n.endswith('measurements.json') and ('signature4' in n or n.startswith('before3111/') or n.startswith('final3111/'))]
 records={}
 for n in selected:
  b=t.extractfile(n).read();records[n]=json.loads(b)
  p=C/'intermediate-raw'/n;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(b)
 before=records['before3111/measurements.json'];after=records['final3111/measurements.json']
 final_by_name={Path(x['source']).name:x for x in after};before_by_name={Path(x['source']).name:x for x in before}
 supplements={n:[dict(source=r['source'],before=before_by_name[Path(r['source']).name]['cli_comparison']['exit'],intermediate=r['cli_comparison']['exit'],final=final_by_name[Path(r['source']).name]['cli_comparison']['exit']) for r in rows] for n,rows in records.items() if 'signature4' in n}
 (C/'intermediate-audit.json').write_text(json.dumps(dict(failure_snapshots=facts,signature_captures=supplements,capture_archive_sha256=sha((M/'captures.tar.gz').read_bytes())),indent=2)+'\n')
print([(x['phase'],x['tier'],len(x['new_losses'])) for x in facts]);print(json.dumps(supplements,indent=2))
