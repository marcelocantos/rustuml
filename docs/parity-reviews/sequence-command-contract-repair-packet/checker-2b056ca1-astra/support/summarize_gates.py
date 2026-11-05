# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,re,subprocess
from pathlib import Path
C=Path(__file__).parent;sha=lambda b:hashlib.sha256(b).hexdigest();s={}
for label in ['before','after']:
 L=C/(label+'-gates');log=(L/'broad.log').read_text();prod=re.search(r'golden_no_oracle: (\d+)/(\d+)',log);ass=re.search(r'golden_pairs: (\d+) total, (\d+) passed, (\d+) failed, (\d+) skipped',log);seq=re.search(r'^  sequence\s+(\d+)/(\d+)',log,re.M)
 tail=log[log.index('held-out no-oracle perturbations diverged'):];failures=set(re.findall(r'^  (test-diagrams/perturbations/[^\n]+?\.puml):',tail,re.M));rev=json.loads((C/(label+'-build.json')).read_text())['revision'];entries=sum(p.endswith('.puml') for p in subprocess.check_output(['git','ls-tree','-r','--name-only',rev,'--','test-diagrams/perturbations'],text=True).splitlines())
 s[label]=dict(revision=rev,product=dict(pass_count=int(prod[1]),denominator=int(prod[2])),assisted=dict(total=int(ass[1]),pass_count=int(ass[2]),fail_count=int(ass[3]),skipped=int(ass[4]),eligible=int(ass[1])-int(ass[4])),sequence=[int(seq[1]),int(seq[2])],standing=dict(entries=entries,failures=len(failures),matches=entries-len(failures),cwd=str(Path.cwd()/'crates/rustuml-oracle')),units=[int(x) for x in re.findall(r'test result: ok\. (\d+) passed;',(L/'units.log').read_text())],cli=[int(x) for x in re.findall(r'test result: ok\. (\d+) passed;',(L/'cli.log').read_text())],commands=json.loads((L/'commands.json').read_text()),stable_broad_runs=1)
 (L/'standing-failure-names.txt').write_text('\n'.join(sorted(failures))+'\n')
s['failure_name_comparison']={}
for fn in ['no_oracle_failure_names.txt','golden_failure_names.txt']:
 b=(C/'before-gates'/fn).read_bytes();a=(C/'after-gates'/fn).read_bytes();s['failure_name_comparison'][fn]=dict(byte_equal=b==a,before_sha256=sha(b),after_sha256=sha(a));assert b==a
s['guard_result']='4/6 each; decimal provenance and review acceptance guards remain red';s['limits']='Main library suite counts are not shipped CLI replay counts. Before standing1724 and after standing2258 differ because imported534 evidence is included; compare explicit fixed3111 CLI cohort for regressions. No broad gate rerun after adding fresh evidence.'
(C/'gates-summary.json').write_text(json.dumps(s,indent=2)+'\n');print(json.dumps({k:{a:b for a,b in v.items() if a!='commands'} if k in ['before','after'] else v for k,v in s.items()},indent=2))
