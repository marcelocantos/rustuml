# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import hashlib,json,re,subprocess
from pathlib import Path
C=Path(__file__).parent;R=Path.cwd();P=R/'docs/parity-reviews/sequence-command-admission-packet/maker-results';J=Path('/Users/marcelo/work/github.com/plantuml/plantuml')
sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*args,cwd=R):return subprocess.check_output(['git',*args],cwd=cwd)
mapping=json.loads((P/'implementation-map.json').read_text());inv=json.loads((P/'inventory.json').read_text());records=[]
for item in inv:
    p=Path(item['java_source']);data=p.read_bytes();rel=str(p.relative_to(J));pinned=git('show','71806a23780b04a5ccde2f8ceb5121edad5eb711:'+rel,cwd=J)
    records.append(dict(index=item['registration_index'],class_name=item['class'],source=rel,sha256=sha(data),matches_inventory=sha(data)==item['java_source_sha256'],matches_pin=data==pinned))
accounts=[]
for m in mapping['mechanisms']:
    p='docs/parity-reviews/'+m['mechanism']+'/account.json'
    accounts.append(dict(m,account_path=p,account_unchanged=git('show',m['account_commit']+':'+p)==(R/p).read_bytes(),precedes=[subprocess.run(['git','merge-base','--is-ancestor',m['account_commit'],commit]).returncode==0 and m['account_commit']!=commit for commit in m['implementation_commits']],messages=[git('show','-s','--format=%B',commit).decode() for commit in m['implementation_commits']]))
diff=git('diff','b4849a500','c525fd32a','--','crates').decode();(C/'production.diff').write_text(diff)
files=git('diff','--name-only','b4849a500','c525fd32a','--','crates').decode().splitlines()
tree_diff=git('diff','--name-only','268fa9f8b','c525fd32a','--','crates','Cargo.toml','Cargo.lock','.cargo','scripts').decode().splitlines()
raw=json.loads((P/'command-inventory-raw.json').read_text())['commands']
source=(R/'crates/rustuml-parser/src/parse/sequence.rs').read_text()
registry=source[source.index('static REGISTERED_SEQUENCE_COMMANDS'):source.index('/// Parse preprocessed')]
rust_classes=re.findall(r'RegisteredSequenceCommand::new\(\s*"([^"]+)"',registry)
probe_classes=[x['class'].split('.')[-1] for x in raw]
pattern_records=[]
for i,chunk in enumerate(registry.split('RegisteredSequenceCommand::new(')[1:]):
    p=re.search(r'r###"(.*?)"###',chunk,re.S)[1]
    signature=int(re.search(r'(\d+)u64',chunk)[1]);original=next(x for x in raw[i]['fields'] if 'pattern' in x)
    pattern_records.append(dict(index=i,class_name=rust_classes[i],rust_pattern=p,java_pattern=original['pattern'],signature=signature,java_signature=int(original.get('fox_signature',0)),support_flag=bool(re.search(r'\btrue,\s*\)',chunk))))
apply=source[source.index('    fn apply_registered('):source.index('    fn parse_line(')]
# Record the real dispatch match arms as independently extracted data, including
# combined participant/lifecycle handlers, rather than trusting maker labels.
arms=[]
for m in re.finditer(r'((?:\s*"[^"\n]+"\s*\|?)+)\s*=>\s*(.*?)(?=\n\s*"|\n\s*_ =>)',apply,re.S):
    arms.append(dict(classes=re.findall(r'"([^"]+)"',m[1]),body=m[2].strip()))
for row in pattern_records:
    row['actual_handler']=next((a['body'] for a in arms if row['class_name'] in a['classes']),None)
    row['maker_handler']=inv[row['index']]['rust_handler']
result=dict(java_revision=git('rev-parse','HEAD',cwd=J).decode().strip(),java_worktree_status=git('status','--porcelain',cwd=J).decode(),source_hashes=records,accounts=accounts,changed_production_files=files,size_revert_remaining_source_diff=tree_diff,registration_count=len(rust_classes),registration_order_equal=rust_classes==probe_classes,registry=pattern_records,dispatch_arms=arms,protected_changes=git('diff','--name-only','dc438fd0','c6295cc','--','bullseye.yaml','test-diagrams/no_oracle_baseline.txt','crates/rustuml-oracle','crates/rustuml-parser/src/parse/mod.rs','crates/rustuml-parser/src/preprocess','crates/rustuml-render/src/svg.rs','crates/rustuml-render/src/text_render.rs').decode().splitlines())
(C/'source-audit.json').write_text(json.dumps(result,indent=2)+'\n')
print('registry',len(rust_classes),'ordered',result['registration_order_equal'],'java mismatches',sum(not r['matches_pin'] or not r['matches_inventory'] for r in records),'source residue',tree_diff,'protected',result['protected_changes'])
