# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,re,subprocess
from pathlib import Path
C=Path(__file__).parent/'independent';R=Path.cwd();summary={};sha=lambda b:hashlib.sha256(b).hexdigest()
for label,rev in [('before','0cde5d7e'),('after','bce6bea')]:
 log=(C/(label+'-main-tiers-and-guards.log')).read_text();u=(C/(label+'-units.log')).read_text();section=log[log.index('held-out no-oracle perturbations diverged'):];failures=sorted(set(re.findall(r'^  (test-diagrams/perturbations/[^\n]+?\.puml):',section,re.M)));paths=subprocess.check_output(['git','ls-tree','-r','--name-only',rev,'--','test-diagrams/perturbations'],text=True).splitlines();entries=sum(p.endswith('.puml') for p in paths)
 prod=re.search(r'golden_no_oracle: (\d+)/(\d+)',log);ass=re.search(r'golden_pairs: (\d+) total, (\d+) passed, (\d+) failed, (\d+) skipped',log)
 summaries=dict(units=[int(x) for x in re.findall(r'test result: ok\. (\d+) passed',u)],ascii=len(re.findall(r'^test ascii::',u,re.M)),cli=int(re.search(r'test result: ok\. (\d+) passed',(C/(label+'-cli.log')).read_text())[1]),product_library=[int(prod[1]),int(prod[2])],assisted_library=dict(total=int(ass[1]),passed=int(ass[2]),failed=int(ass[3]),skipped=int(ass[4])),sequence=re.search(r'^  sequence\s+(\d+/\d+)',log,re.M)[1],standing=dict(entries=entries,failed=len(failures),matched=entries-len(failures),cwd=str(R/'crates/rustuml-oracle')),guards='4/6, red on decimal provenance and review records',renderer_clippy_errors=int(re.search(r'due to (\d+) previous errors', (C/(label+'-clippy-render.log')).read_text())[1]))
 (C/(label+'-standing-failures.txt')).write_text('\n'.join(failures)+'\n');summary[label]=summaries
summary['failure_names']={}
for n in ['no_oracle_failure_names.txt','golden_failure_names.txt']:
 b=(C/'before-failure-names'/n).read_bytes();a=(C/'after-failure-names'/n).read_bytes();m=gzip.decompress((R/'docs/parity-reviews/sequence-filtered-display-packet/maker-results/logs/final-failure-names'/(n+'.gz')).read_bytes());summary['failure_names'][n]=dict(before_sha256=sha(b),after_sha256=sha(a),before_after_equal=a==b,maker_final_equal=a==m)
summary['setup_failure']='Initial submodule update failed because configured local file transport was disabled; preserved log; command-scoped protocol.file.allow=always retry hydrated exact pinned submodule. No gate rerun.'
summary['commands']=json.loads((C/'commands.json').read_text());(C/'broad-summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps({k:v for k,v in summary.items() if k!='commands'},indent=2))
