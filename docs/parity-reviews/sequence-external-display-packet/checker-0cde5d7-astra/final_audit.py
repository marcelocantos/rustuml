# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import gzip,hashlib,json,subprocess
from pathlib import Path
C=Path(__file__).resolve().parent;R=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-5fc3056e');M=R/'docs/parity-reviews/sequence-external-display-packet/maker-results'
readgz=lambda p:json.loads(gzip.decompress(p.read_bytes()))
ours=readgz(C/'replay2281/measurements.json.gz');out={}
for rev,file in [('before','before-measurements.json.gz'),('after','measurements.json.gz')]:
 maker=readgz(M/file);assert len(maker)==len(ours)==2281
 out[rev]=dict(entries=2281,source_hash_differences=[i for i,(o,m) in enumerate(zip(ours,maker)) if o['source_sha256']!=m['source_sha256']],reference_hash_differences=[i for i,(o,m) in enumerate(zip(ours,maker)) if o['golden_sha256']!=m['golden_sha256']],svg_hash_differences=[i for i,(o,m) in enumerate(zip(ours,maker)) if o['results'][rev]['cli']['sha256']!=m['cli']['svg_sha256']],status_differences=[i for i,(o,m) in enumerate(zip(ours,maker)) if o['results'][rev]['cli']['exit']!=m['cli']['exit']],comparison_differences={mode:[i for i,(o,m) in enumerate(zip(ours,maker)) if o['results'][rev][mode]['exit']!=m[mode]['exit']] for mode in ['cli_comparison','product_library','assisted_library']})
parent=readgz(R/'docs/parity-reviews/sequence-rejection-repair/checker-support/replay1449.json.gz');paths={r['source'] for r in ours};overlap=[p for p in parent if p['original_source'] in paths];unreplayed=[p for p in parent if p['original_source'] not in paths]
out['parent1449']=dict(entries=len(parent),overlap_paths=len(overlap),unreplayed_paths=len(unreplayed),overlap_unique_bytes=len({p['source_sha256'] for p in overlap}),unreplayed_unique_bytes=len({p['source_sha256'] for p in unreplayed}),exclusive_unreplayed_unique_bytes=len({p['source_sha256'] for p in unreplayed}-{p['source_sha256'] for p in overlap}))
fresh=readgz(C/'fresh60/measurements.json.gz')+readgz(C/'followup4/measurements.json.gz');out['fresh_source_overlap']=sorted({r['source_sha256'] for r in fresh}&{r['source_sha256'] for r in ours});out['fresh_unique']=len({r['source_sha256'] for r in fresh})
out['ast_ascii_status_changes']={mode:[r['source'] for r in ours if any(r['results']['before'][mode][k]!=r['results']['after'][mode][k] for k in ['exit','stderr','sha256'])] for mode in ['ast','ascii']}
# Binding remains the exact privately built releases; comparator source is unchanged.
out['comparator_source_diff']=subprocess.check_output(['git','diff','653b1ed7','d714bb55','--','crates/rustuml-oracle/src/compare.rs','crates/rustuml-oracle/src/harness.rs'],cwd=R,text=True)
out['comparator_note']='Same adapter source compiled against each exact release rlib; binary hashes differ because releases were built independently in different private targets. No comparator algorithm, slack, skip rule or allowance changed.'
out['adapter_sha256']=hashlib.sha256((C/'compare_cli.rs').read_bytes()).hexdigest()
out['adapter_imported_unchanged']=(C/'compare_cli.rs').read_bytes()==(R/'docs/parity-reviews/sequence-dependency-closure/checker-support/compare_cli.rs').read_bytes()
(C/'maker-reproduction-and-overlap.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out,indent=2))
