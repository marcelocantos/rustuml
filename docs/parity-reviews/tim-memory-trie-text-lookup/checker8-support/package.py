# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Register immutable checker evidence; never modify historical evidence."""
import gzip
import json
import shutil
import subprocess
import tarfile
from pathlib import Path
from replay import R, S, dump, sha

candidate='faf463445591a53f17d25c5c16611f8e626ef93d'
assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==candidate
fresh=json.loads((S/'fresh-results.json').read_bytes())
standing={Path(r['source']).stem:r for r in json.loads((S/'fresh-standing.json').read_bytes())}
summary=json.loads((S/'summary.json').read_bytes())
support=R/'docs/parity-reviews/tim-memory-trie-text-lookup/checker8-support'
assert not support.exists()
support.mkdir()
audit={
 'reviewer': {'task_id':'thread:delegated-task:command%3Amcp%3A7a6e5142-c330-4dbe-941b-87119bcc7cef%3Adelegate-task%3A78227ae1-6e7a-4609-acf0-9f2049302acc', 'model':'gpt-6-astra/high', 'tool':'Codex harness through T3 Code; exec_command/rustc/cargo/Python; parent Monkey Man relay only', 'checkout':str(R), 'branch':'review/tim8-faf4634-independent-20261104', 'independence':'This task implemented none of the candidate; no children, Java/javac, own SSH, production edits, target edits or shipping.'},
 'candidate':candidate,
 'classification':'Reference comparable (oracle-first class 2 reduced to machine comparator/source diagnostics). Verification setup and process provenance are the dominant costs. Shipped CLI decides SVG claims; direct source and library diagnostics are separate.',
 'read_scope':['AGENTS.md','docs/parity-reviews/README.md','entire crates/rustuml-parser/src/preprocess/mod.rs (5759 lines)','docs/parity-reviews/tim-memory-trie-text-lookup/maker-support/README.md','oracle-first SKILL.md','Rust/Python companions','Java sources hashed in integrity.json'],
 'trie':[
  'Java TrieImpl.add appends a NUL sentinel; Rust NameTrie.terminal represents that edge. Traversal returns the current terminal at EOF, a missing child, or a dead child. It never saves a prior terminal. Rust child.children.is_empty() && terminal.is_none() is equivalent to Java child.brothers.size()==0 for admitted non-NUL representable keys.',
  'Both remove operations clear only the terminal sentinel and retain all path nodes. A removed leaf is a dead child, so its parent may still match; a retained internal path can prevent backtracking. Fresh terminal_history challenges removed leaf/internal path/reinsertion; original maker prefix_paths and removal_paths are retained and rerun. ASCII boundaries can mask some retained-node differences, so these controls do not independently prove every trie-state distinction.',
  'TMemoryGlobal and automatic TMemoryLocal locals construct eager tries. Override/argument map starts without a trie and is indexed on first variablesNames3. VariableMemory::names and LocalMemory::default implement that distinction. Fresh lazy_before/lazy_after exercise scan timing, but ordinary identifier boundaries can make the outputs coincide; the maker unit-state probes are supplementary model evidence, not shipped-path completeness.',
  'All production mutation sites were audited: set_var (override/existing global/automatic local), set_local_var, remove_var (override/global/local), try_define, explicit global assignment, formal binding, foreach global binding, and SetVariableValue GLOBAL. They route through VariableMemory insert/remove. The values field is private and directly accessed only by its wrapper. Each function frame reads only its own overrides/locals plus global; sibling function calls fork from global.',
  'variable_name_at asks global, override and automatic-local tries, chooses the longest returned prefix before the right boundary, then get_var chooses override/global/local value precedence. Length ordering is identical in UTF-8 and UTF-16 for prefixes of the same valid string. Tied prefixes have the same key. Fresh punctuation_json verifies that a stored $j.leaf key beats JSON member syntax, while $j[leaf] still traverses JSON.',
  'VariableManager boundaries use original input positions, literal backslash-n exception and dollar-left-boundary exception. Rust scans original offsets and never rescans appended values. concat_original checks generated ## output, raw stored function-looking text and function priority, including a stored read() key. Java also removes ## from accumulated result, not merely from original text; this is intentional.',
  'Sprite-markup shortcut removal lets original stored names substitute inside markup while inserted literals remain unscanned. Fresh markup_literal has exact evaluated source but strict SVG failure (13 Java versus 10 Rust elements). This reproduces existing maker markup_scan renderer ownership; no source-equivalence waiver applies.',
  'NUL rejection now prevents SetVariableValue insertion and propagates the existing fatal channel. Java mutates its value map before TrieImpl.add rejects NUL; execution aborts, so subsequent source effects do not observe the map. No claim is made about post-fatal internal memory introspection.'
 ],
 'binding':[
  'Eater.eatDeclareFunction stores an ordered argument list and evaluates each default at declaration. TFunctionImpl signature carries distinct names but preserves full declared arity; getNewMemory iterates the ordered list and overwrites the exact map key on each occurrence.',
  'Java namedArguments.containsKey/get is non-consuming. Rust named.get(...).cloned() matches it, retaining TValue kind (including numeric-looking strings and JSON); only or_else(positional.next) advances the positional cursor. Captured default clones remain the final fallback. No selection, arity or named-map-lifetime changes were introduced by TIM8.',
  'Fresh duplicate_typed, duplicate_positions, duplicate_effect_order and duplicate_required all match source and strict CLI SVG. They vary JSON/string/integer values, named/positional/default paths, repeated named RHS effects, bare/dollar identity, formal ordering and required duplicate formals. All four failed SVG before and pass after.',
  'EaterFunctionCall evaluates named RHS arguments in order into one map; duplicate argument names still execute each RHS once before final overwrite. RPN has a separate expression-wide named map and subtracts distinct key count before canCover(empty names). Fresh duplicate_negative intentionally triggers that existing required-arity boundary: Java exit 200 and Rust exit 1 reject it. It is not a positive reference and error-output/source parity is not established.',
  'Java negative SVG includes welcome text as a background to its diagnostic. The initial checker classifier called that welcome; it was corrected using exit 200/ERROR stderr, retaining the initial classifier, initial result and correction record without rerunning product or reference.'
 ],
 'iterator':[
  'TContext.buildCodeIterator orders comments -> CodeIteratorSub -> return/procedure collectors -> conditional -> legacy define -> while -> foreach -> assignment. An empty replay installed in CodeIteratorSub.peek returns null; executeLines does not call next, so the replay never clears and the whole current iterator ends.',
  'Rust collects sub before function/loop/condition execution, sets iterator_stopped on an empty collected vector, and checks it at top-level, buffered function and loop boundaries. process_lines/process_buffered_lines save and restore the outer stop value for nested executeLines lifetimes. Replay inside the current iterator uses process_buffered_line directly. Nonempty replay and original source origins remain in that iterator.',
  'Fresh empty_false and empty_loop prove that inactive conditional/zero-iteration loop context does not bypass the upstream empty collector. Exact source is the executed prefix with no end tag and no Tail. Both Java results are welcome; Rust still chooses Class. These are whole-product failures despite exact source.',
  'Fresh empty_two supplies two model lines before a comment-only empty collector. It is a positive CLASS reference and now matches CLI SVG as well as source. Fresh blank_replay distinguishes one blank line from an empty iterator and exercises includesub replay; it passes.',
  'Fresh empty_nested calls an include inside a procedure; the included outer file calls a second include with an empty sub, then resumes its own effects and returns to the procedure and outer caller. empty_return_include resumes a return function after its included iterator stops. empty_selected tests a genuinely empty selected external body without treating it as a collected-sub terminator. All three pass CLI SVG with requested cwd/resources.',
  'The unchanged standing harness passes None for include base and runs from its crate directory. The focused unchanged-algorithm slice gets 13/20 rather than CLI 16/20: three extra failures are empty_nested, empty_return_include and empty_selected with unresolved include resources. Resources and failures are committed; no harness/cwd policy was edited or waived.',
  'Existing maker empty_include remains a valid whole-product failure: evaluated source is exact and source_line 0 correctly comes from included Java origin; renderer class.rs uses entity_index+1 for zero. Existing welcome factory selection remains a separate parse/PSystemBuilder ownership issue. No artificial end tags, reader normalization or renderer edits were made.'
 ],
 'known_unproved_limits':[
  'Rust regex Unicode letter tables are not certified identical to the pinned JDK Character.isLetter version. The valid astral_offsets control exercises full surrogate pairs only. Unicode-version/property coverage is not established by these few cases.',
  'General isolated-surrogate text/name projection remains unproved: UTF-16 TValue units can survive intermediate expressions but name/display conversion uses lossy UTF-8 projection. This known boundary is not reclassified as a new TIM8 mechanism defect, nor certified silently.',
  'Function discovery retains its pre-existing scanner and Unicode predicates, unlike the new stored-name variable trie. Universal function-name trie/Unicode boundary parity is not certified. Tests establish priority only on the specific admitted ASCII function controls.',
  'JSON member Character.isJavaIdentifierPart versus Rust token classification, signature tree-bin/JVM iteration, legacy function syntax and occupancy, Unicode IntVal, raw-reader GET-versus-pipe semantics, renderer error pages/origins/class/style/sequence geometry and environment-dependent builtin behavior remain outside this corrective acceptance claim.',
  'No global environment equivalence: reference captured Java 21.0.5/Graphviz 12.2.0 en_AU UTF-8 GMT+11, while historical origins differ. The wrapper/JAR/resources and each recorded process contract were verified only within their stated scope.'
 ],
 'source_derived_next_steps':{
  'tim-memory-trie-text-lookup':['Resolve unresolved sprite marker projection in the downstream Creole/sprite renderer, using fresh markup_literal and preserved maker markup_scan; do not rescan inserted values.','Keep original boundary predicates and trie history independent. Any future Unicode acceptance requires pinned Character property/version evidence and explicit surrogate-domain contract, not ASCII sample inference.'],
  'tim-duplicate-formal-binding':['Retain non-consuming typed get and positional cursor semantics. No valid binding regression or new binding defect was found.','Preserve duplicate_negative as a negative/error-output boundary: both reject, but Java retains the failing line in diagnostic source and produces an error SVG; Rust exits without matching that output. No builtin/signature expansion was attempted.'],
  'tim-empty-sub-iterator-termination':['Route the already-exact stopped evaluated source through equivalent PSystemBuilder/PSystemWelcomeFactory selection (two total source lines), without manufacturing end tags/comments.','Correct downstream class source-line zero handling for preserved empty_include. Resolve standing include-base/environment ownership separately, preserving all current failures and resource hierarchy.']
 }
}
dump(S/'source-audit.json',audit)
for row in fresh:
    p=Path(row['row']['source']);m=row['row']['mechanism'];dest=R/'test-diagrams/perturbations'/m/'checker8'
    dest.mkdir(parents=True,exist_ok=True)
    assert not (dest/p.name).exists()
    shutil.copy2(p,dest/p.name);shutil.copy2(S/'fresh-reference'/p.stem/'java.svg.stdout',dest/p.with_suffix('.svg').name)
    if m=='tim-empty-sub-iterator-termination':
        for resource in (S/'new').glob('*.iuml'):shutil.copy2(resource,dest/resource.name)
mechanisms=['tim-memory-trie-text-lookup','tim-duplicate-formal-binding','tim-empty-sub-iterator-termination']
reason={
 mechanisms[0]:'Rejected for whole-output acceptance: fresh markup_literal remains a valid strict SVG counterexample, reproducing the preserved unresolved-sprite rendering failure. The bounded text-lookup correction survived these valid source controls; no new in-scope trie defect was demonstrated.',
 mechanisms[1]:'Rejected for unqualified whole-output acceptance, not a claimed new binding defect: all four fresh valid binding controls pass CLI SVG and exact source, while the preserved negative control produces unmatched Java diagnostic output and the standing/guard acceptance gates remain red. Correct rejection of an invalid input is recorded separately from whole-output parity.',
 mechanisms[2]:'Rejected for whole-output acceptance: fresh empty_false/empty_loop and retained welcome truncations still select the wrong downstream factory; maker empty_include remains a source-line-zero renderer failure. Empty-iterator source behavior survived the valid controls, including nested caller continuation.'
}
commands=[f'CARGO_TARGET_DIR={S}/target-after CARGO_BUILD_JOBS=4 cargo build -p rustuml --release',f'git switch --detach 2cf6c7eeebae7354e7a3b7e53b86fb3754883e67; CARGO_TARGET_DIR={S}/target-before CARGO_BUILD_JOBS=4 cargo build -p rustuml --release; git switch review/tim8-faf4634-independent-20261104',f'uv run --no-project python {S}/gates.py',f'uv run --no-project python {S}/replay.py before',f'uv run --no-project python {S}/replay.py after',f'uv run --no-project python {S}/fresh.py',f'uv run --no-project python {S}/summarize.py','Remote SVG/probe argv/cwd/env for all 40 calls: checker8-support/cases/*/commands.json; immutable remote-request.json specifies only approved reference wrapper and <worker> placeholders.']
for mechanism in mechanisms:
    perturbations=[]
    selected=[r for r in fresh if r['row']['mechanism']==mechanism]
    for row in selected:
        name=Path(row['row']['source']).stem;relative='test-diagrams/perturbations/'+mechanism+'/checker8/'+name
        perturbations.append(dict(source=relative+'.puml',golden=relative+'.svg',axes=row['row']['axes'],
            result='pass' if row['reference_valid'] and row['phases']['after']['svg_match'] and standing[name]['pass_status'] else 'fail',
            intended_validity=row['row']['intended_validity'],observed_reference_category=row['classification'],
            reference_valid=row['reference_valid'],java_svg_exit=row['java_svg_exit'],java_probe_exit=row['java_probe_exit'],
            cli_svg_match=row['phases']['after']['svg_match'],cli_source_exact=row['phases']['after'].get('exact_source'),
            direct_source_exact=row['direct_source_equal'],standing_algorithm_slice_exit=standing[name]['call']['exit'],
            evidence='docs/parity-reviews/tim-memory-trie-text-lookup/checker8-support/cases/'+name+'/result.json'))
    record=dict(schema_version=1,mechanism=mechanism,reviewer=audit['reviewer'],account_commit='02fa49de2396c1445a4ddf1a783c973f883a82f4',
       implementation_commit='2274e808433568aa1d8042bf9e3cb1f1de338b4b',candidate_revision=candidate,before_revision='2cf6c7eeebae7354e7a3b7e53b86fb3754883e67',
       java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',java_jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',
       commands=commands,perturbations=perturbations,findings=[reason[mechanism],
        'Independently reproduced retained1672: 1560 valid, 665 SVG before and after, 158/160 available valid sources exact. Total1710 entries/1631 distinct pairs/79 duplicate entries; no valid losses, new errors or valid model drops.',
        'Fresh20 outcomes: 17 valid/16 CLI SVG matches, one syntax negative, two welcome truncations. Every nonnegative evaluated source is exact; negative diagnostic source is not. Original intended outcomes are retained separately.',
        'Required exact-candidate gates ran once: parser622/CLI15 pass; main product11179/11251 and assisted10497/11251 exit101; standing609/1628 versus matching-environment CLI636 with27 discrepancies; guards4/6 and existing component dead-code clippy exit101. Touched rustfmt/diff pass. No gate/baseline/policy changes.',
        'All9245 imported historical evidence files and129 original preprocessing test blocks unchanged. All19766 maker archive files and169 package hash entries verified, including failed iterations. Account chronology independently verified before code edits.',
        'No unqualified Unicode-version/property/isolated-surrogate or global environment equivalence claim. See source-audit.json for exact boundary and next-step ownership.'],
       verdict='rejected',bounded_mechanism_result='No new in-scope causal defect demonstrated on tested valid inputs; whole-output acceptance is not granted.',
       next_steps=audit['source_derived_next_steps'][mechanism],evidence_root=str(support.relative_to(R)))
    p=R/'docs/parity-reviews'/mechanism/'review.json';assert not p.exists();dump(p,record)
# Fresh raw streams and metadata are directly readable; full local captures stay in the archive.
shutil.copytree(S/'fresh-reference',support/'cases')
for name in ['integrity.json','historical-files.json','maker-archive-validation.json','maker-archive-files.json','build-provenance.json','comparator-build.json','source-probe-build.json','standing-slice-build.json','summary.json','nonpositive-statuses.json','standing-mapping.json','fresh-results.json','fresh-standing.json','remote-validation.json','source-audit.json','controls.json','classification-correction.json','gates.json','remote-request.json','new-manifest.json','parent-relay-validation.json','reference-results.json.gz','reference-transport.json','transfer-provenance.json','prepare.py','integrity.py','replay.py','summarize.py','fresh.py','gates.py','source_probe.rs','standing_slice.rs','package.py']:
    shutil.copy2(S/name,support/name)
shutil.copytree(S/'logs',support/'logs')
excluded={'target-before','target-after','before-rustuml','after-rustuml','probe','source-probe','standing-slice','__pycache__'}
inventory=[]
with (support/'measurements.tar.gz').open('wb') as raw:
    with gzip.GzipFile(fileobj=raw,mode='wb',mtime=0) as gz:
        with tarfile.open(fileobj=gz,mode='w|') as archive:
            for path in sorted(S.rglob('*')):
                rel=path.relative_to(S)
                if any(x in excluded for x in rel.parts) or not path.is_file():continue
                data=path.read_bytes();inventory.append(dict(path=str(rel),bytes=len(data),sha256=sha(data)))
                archive.add(path,arcname=str(rel),recursive=False)
dump(support/'archive-files.json',inventory)
dump(support/'archive-integrity.json',dict(files=len(inventory),sha256=sha((support/'measurements.tar.gz').read_bytes()),excluded=sorted(excluded),private_cache=str(S)))
print(support, 'archive files',len(inventory),'bytes',(support/'measurements.tar.gz').stat().st_size)
