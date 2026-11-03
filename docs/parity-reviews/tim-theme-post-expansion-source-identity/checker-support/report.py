# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import collections
import difflib
import hashlib
import json
from pathlib import Path
import shutil

root = Path('/tmp/rustuml-tim-checker')
repo = Path.cwd()
mechanism = 'tim-theme-post-expansion-source-identity'
record = repo/'docs/parity-reviews'/mechanism
support = record/'checker-support'
support.mkdir(exist_ok=True)
cases = json.loads((root/'cases.json').read_text())
before = {Path(r['source']).parent.name:r for r in json.loads((root/'before-fresh.json').read_text())}
after = {Path(r['source']).parent.name:r for r in json.loads((root/'after-fresh.json').read_text())}
metadata = {r['name']:r for r in json.loads((root/'metadata.json').read_text())}

for script in ['probe.rs','SourceProbe.java','generate.py','add_cases.py','measure.py','metadata.py','report.py']:
    shutil.copyfile(root/script,support/script)

rows = []
for name, info in cases.items():
    directory = root/'cases'/name
    dest = support/'cases'/name
    dest.mkdir(parents=True,exist_ok=True)
    for file in ['before.svg','after.svg','before.expand','after.expand','java.expanded','before.uml-source','after.uml-source','java.uml-source','before.lines','after.lines','java.stderr','preproc.stderr','file-context.expanded']:
        if (directory/file).exists(): shutil.copyfile(directory/file,dest/file)
    if 'maker' in info: shutil.copyfile(directory/'java.svg',dest/'fresh-java.svg')
    for phase in ['before','after']:
        result = before[name] if phase=='before' else after[name]
        (dest/f'{phase}.diff.txt').write_text(result.get('comparison',result.get('stderr','')))
        diff = difflib.unified_diff((directory/'java.uml-source').read_text().splitlines(True),(directory/f'{phase}.uml-source').read_text().splitlines(True),fromfile='Java UmlSource',tofile=f'{phase} CLI metadata source')
        (dest/f'{phase}.source.diff.txt').write_text(''.join(diff))
    row = {'source':f'test-diagrams/perturbations/{mechanism}/{name}.puml','golden':f'test-diagrams/perturbations/{mechanism}/{name}.svg','axes':info['axes'],'result':after[name]['result'],'before_result':before[name]['result'],'valid_java_reference':after[name].get('comparison_exit')!=2,'java_svg_exit':info['java_exit'],'before_cli_exit':before[name]['cli_exit'],'after_cli_exit':after[name]['cli_exit'],'before_cli_source_equal':metadata[name]['before_equal'],'after_cli_source_equal':metadata[name]['after_equal'],'before_generic_expansion_equal':before[name]['expansion_equal'],'after_generic_expansion_equal':after[name]['expansion_equal'],'evidence':str(dest.relative_to(repo))}
    if 'maker' in info:
        row['maker_original_source_sha256']=hashlib.sha256((Path(info['maker'])/'source.puml').read_bytes()).hexdigest()
        row['maker_original_svg_sha256']=hashlib.sha256((Path(info['maker'])/'source.svg').read_bytes()).hexdigest()
        row['fresh_java_matches_preserved_strict']=True
    rows.append(row)

isolated = {}
for phase in ['before','after']:
    data = json.loads((root/f'{phase}-isolated.json').read_text())
    groups = collections.defaultdict(list)
    ledger = []
    for result in data:
        path = Path(result['source'])
        group = {'class-native-container-factory-precedence':'class_additions','sequence-grouped-note-origin':'sequence_additions',mechanism:'this_review'}.get(path.parent.name,'original')
        groups[group].append(result)
        ledger.append({'source':str(path.relative_to(repo)),'result':result['result'],'cli_exit':result['cli_exit'],'comparison_exit':result.get('comparison_exit'),'stderr':result['stderr'],'comparison_sha256':hashlib.sha256(result.get('comparison','').encode()).hexdigest()})
    isolated[phase]={name:{'total':len(group),'pass':sum(r['result']=='pass' for r in group),'cli_error_or_panic':sum(r['cli_exit']!=0 for r in group),'invalid_reference':sum(r.get('comparison_exit')==2 for r in group)} for name,group in groups.items()}
    (support/f'{phase}-isolated-ledger.json').write_text(json.dumps(ledger,indent=2)+'\n')

findings = [
 'REJECTED. The unchanged strict comparator rejects a new shipped-CLI regression, 15 valid fresh checker cases fail in total, and both previously disclosed maker failures persist. Improvements in expansion alone are not an acceptance waiver.',
 'New regression: checker_sprite_identity passed the baseline CLI and fails the candidate solely on two gradient IDs and their references (five attribute mismatches). Java retains the four-line sprite declaration in UmlSource; process_one_line emits four blank lines even in source_identity_mode. The old raw themed reconstruction accidentally preserved it. Java prefix xodtwgpqwe1m becomes Rust 11q993tevjnoy; geometry/colors/text are otherwise comparator-equal. Preserve actual sprite text in identity without changing parser sprite extraction.',
 'Theme selector evaluation is missing: checker_theme_variable and checker_theme_function are valid Java but render unthemed in Rust. EaterTheme.analyze uses TContext.applyFunctionsAndVariables on the name (and from expression); Rust try_theme directly calls get_theme_source(name_part). Both baseline and candidate fail. This is an unresolved TIM dependency directly within the claimed active-theme expansion invariant.',
 'User subblock execution is missing: checker_subblock_state loses the classBorderColor #123456 line and its assignment in identity and parser output. CodeIteratorSub.peek replays body lines at the definition; Rust expand_subs_inline is enabled only during a theme load, so top-level sub bodies remain buffered. The SVG border stays #2FA4E7. Do not repair this by inserting a hash-only string; execute the state transition in the real parser stream too.',
 'Consumed debug directive leaks into identity: checker_dump_memory adds a synthetic apostrophe [dump] line to Rust UmlSource. Java TContext.executeDumpMemory / EaterDumpMemory only dumps diagnostic memory, adding no resultList line. The resulting CLI gradient IDs differ. Both revisions fail; the candidate still violates directive-consumption semantics.',
 'Comment/newline semantics remain incomplete: valid checker_inner_comment retains an opening inline block-comment fragment in Rust; StringLocated.removeInnerComment removes the trailing block in Java. checker_blank_lines drops two raw initial empty lines that the fresh Java SourceStringReader UmlSource retains, changing the seed. These mismatches were confirmed with actual Java UmlSource and Rust CLI --yaml metadata, not only -preproc or a generic helper.',
 'Procedure-local scope differs: checker_theme_procedure expands theme helper variables to concrete colors in Rust, while fresh Java retains $PRIMARY_TEXT/$PRIMARY inside nested helper calls. TFunctionImpl.getNewMemory uses memory.forkFromGlobal, and TMemoryLocal.forkFromGlobal creates a sibling scope from global memory; Rust call_function/set_var places new ordinary assignments in defines, visible to nested calls. Both revisions fail the CLI. Port the memory visibility rule before claiming themes in procedures generalize.',
 'Guarded theme state differs: checker_outline_state supplies BGCOLOR=#123456, but Rust changes it to transparent. The theme calls %variable_exists("$BGCOLOR"); Rust lookup checks defines with the dollar-prefixed name, while defines stores bare names. The candidate identity and the parser both disagree with Java. The previously disclosed inactive theme-subblock replay is another remaining parser-stream issue; this checker did not claim it repaired.',
 'Confirmed source-location defect: maker_inactive_theme is Java line 6 versus Rust 5. checker_inactive_theme_pair amplifies it to 6 versus 4, and checker_option_inactive is 5 versus 4. try_theme/try_option produce placeholders, but process_one_line discards their returned vectors when is_active is false. Identity is exact in all three; parser source origins must survive inactive consumed directives.',
 'Confirmed class dependency: maker_repeated_themes leaves the cerulean circled-character palette after minty. checker_repeated_reverse leaves the minty palette after cerulean. Identity is exact in both directions. collapse_reassigned_skinparams retains separate theme-segment keys; ClassFontOverrides::from_skinparams uses forward find for stereotypeCBackgroundColor and stereotypeCBorderColor, and from_diagram_entity does not replace those fields via the current spot style. Java EntityImageClassHeader.getCircledCharacter resolves the current style builder. This belongs to the class/style worker, remains failing, and is not waived here.',
 'Non-themed option control confirms a bounded improvement: checker_option_false now matches Java identity and CLI shadow IDs. checker_option_true now has exact identity but still emits rects where Java handwritten rendering emits polygons. That shape dependency remains a failure. No hand-drawing/comparator changes were made.',
 'Ordering controls confirm real gains: fixed-name themes in foreach/while, inactive function redefinition, an active include, an inactive include, theme variables read through a custom function, and the year clock case have exact candidate CLI source. The inactive function redefinition is not reported as a defect: Java function iterators precede CodeIteratorIf, and fresh Java agrees. Identity correctness must follow actual iterator order, not intuitive conditional semantics.',
 'Sequence diagnostic distinction: generic preprocess_full output leaves the continued line split, but the shipped CLI metadata is exactly Java after sequence parsing enables ending-backslash collapse. checker_line_continuation still fails its SVG (30 Java versus 54 Rust elements); this is downstream rendering/parsing residue, not evidence that the final CLI seed string is wrong.',
 'Clock/environment audit: each PreprocessContext::new independently calls RenderClock::from_env; preprocess_full_inner evaluates parser and identity with separate contexts, and sequence dispatch preprocesses again. Thus the same expression can be sampled at distinct instants and includes can be read multiple times. The stable year control passes under the recorded 2026 pinned Rust clock. No clock-boundary race or concurrent file-change race was empirically forced, so those are explicit unverified risks rather than claimed observed failures. Independently, checker_fixed_timestamp is a deterministic counterexample: Java DateFunction honors its second seconds-since-epoch argument and yields 2000; Rust ignores it and yields 2026. Capture/reuse actual evaluation output/state for identity and honor explicit timestamp arguments; merely sharing a clock would not resolve the other state divergences.',
 'Provenance audit found no new production numeric constants, fixture-keyed production conditions, corpus reads, changed comparator settings or new allow-lists in the candidate diff. New identity branches cite TContext/UmlSource/CodeIteratorSub/executeOption. Existing UTF-16 wrapping 31x hash provenance is StringUtils.seed; resource paths are parse_with_base -> meta.source -> filter_registry and class_gradients, plus activity __svgIdSeed. The model citations describe a legitimate intended correction, but do not establish full execution equivalence.',
 'All eight fresh maker inputs and SVGs were copied byte-for-byte with maker_ prefixes; the additional maker preproc_complex_theme_override corpus diagnostic was also preserved (nine maker pairs total). Freshly regenerated Java SVGs are valid and strict-equivalent to each preserved maker SVG. None of the maker inputs, account, results or earlier reviews was edited.',
 'Unexpected invalid reference retained: checker_inline_comment attempted a trailing single-quote comment after a class declaration. Java produced an error SVG despite exit 0. It remains in the executable corpus and is recorded fail/invalid, excluded only from explicitly labeled valid-reference diagnostic denominators. A separate valid inline block-comment case was added; the original invalid input was not removed or silently replaced.',
 'The original 810 held-outs remain 349 pass, 454 mismatches and 7 invalid references; the 23 class additions remain 0 pass; the 41 sequence additions remain 17 pass, 22 mismatches and 2 real CLI panics. No pre-existing isolated case changes status. The built-in suite instead suffers 378 collateral non-reentrancy panics before and 410 after, following its initiating sequence panic; isolated CLI results are the trustworthy case counts. Neither registry nor harness was changed.'
]

review={
 'schema_version':1,'mechanism':mechanism,
 'reviewer':{'task_id':'t3-9e4e3ea2-independent-tim-source-identity-checker','model':'gpt-6-astra (high), Codex harness in T3 Code','independent':True,'implemented_candidate':False,'children':0},
 'account_commit':'ee8f405de889a60f4679939a2671d3aba28f05db',
 'implementation_commit':'0f2fb90c656b3c575db20a0b3a6d690fdfa33d30',
 'java_revision':'71806a23780b04a5ccde2f8ceb5121edad5eb711',
 'tested_revisions':{'before':'b15a5fedd84b42a913a4f1d2cdfd7747a0c4b094','after':'d8e4741310c95b9cd85dbd168891773d55e7ad85','actual_account':'de8cbb5856383960b09bb4308414304de7aaf41b','actual_implementation':'6aa84d1d1433dc3c995046ec6d9847c74f2ad44d','original_maker_evidence':'a3082894f29e642f146bf2bc573e33df39753e51','actual_maker_evidence':'d8e4741310c95b9cd85dbd168891773d55e7ad85','candidate_production_diff':'crates/rustuml-parser/src/preprocess/mod.rs only'},
 'reference':{'jar':'/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar','jar_sha256':'aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053','stdlib_revision':'747d1eae809cde20675976d67e3eb84dc0021a0c','golden_revision':'9efdc12fb057bbfecb6a8e2d25d31ec1e5ead19a','java':'Temurin 21.0.13, absolute binary under /Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home','historical_binary_identical':False,'historical_fonts_graphviz_equivalence_proven':False},
 'environment':{'CARGO_TARGET_DIR':'/tmp/rustuml-tim-checker/target','CARGO_BUILD_JOBS':'4','PLANTUML_STDLIB':'/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib','RUSTUML_DEBUG':'date=1774210426000,tz=AEDT+1100','fresh_java_date_control':'Year-only 2026; explicit timestamp case uses 946684800 seconds (2000).'},
 'commands':[
  'git cherry-pick ee8f405de889a60f4679939a2671d3aba28f05db 0f2fb90c656b3c575db20a0b3a6d690fdfa33d30 a3082894f29e642f146bf2bc573e33df39753e51',
  'git -c protocol.file.allow=always submodule update --init test-diagrams/golden',
  'CARGO_TARGET_DIR=/tmp/rustuml-tim-checker/target CARGO_BUILD_JOBS=4 cargo check --workspace',
  'git switch --detach b15a5fedd84b42a913a4f1d2cdfd7747a0c4b094; cargo test --release --test golden_no_oracle --test golden_pairs --test parity_perturbations --test harness_guards --no-fail-fast; cargo build --release -p rustuml (recorded environment)',
  '/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java -Djava.awt.headless=true -jar /Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar -pipe -tsvg < source.puml > java.svg',
  'java (absolute executable above) -Djava.awt.headless=true -jar <pinned JAR> -preproc /tmp/rustuml-tim-checker/cases/<name>/source.puml; include-context cases corrected using -pipe -preproc from repository root, original file-context output retained',
  'uv run --no-project python /tmp/rustuml-tim-checker/measure.py before fresh; uv run --no-project python /tmp/rustuml-tim-checker/measure.py before isolated',
  'uv run --no-project python /tmp/rustuml-tim-checker/generate.py; uv run --no-project python /tmp/rustuml-tim-checker/add_cases.py (all generated inputs and references preserved, including the unexpected invalid reference)',
  'git switch t3/review-tim-source-identity; cargo test --release -p rustuml-parser --lib; cargo test --release -p rustuml --test pipe_mode (recorded environment)',
  'cargo test --release --test golden_no_oracle --test golden_pairs --test parity_perturbations --test harness_guards --no-fail-fast (recorded environment, after preserving all 33 pairs)',
  'uv run --no-project python /tmp/rustuml-tim-checker/measure.py after fresh; uv run --no-project python /tmp/rustuml-tim-checker/measure.py after isolated',
  'uv run --no-project python /tmp/rustuml-tim-checker/metadata.py (javac SourceProbe against pinned JAR; actual Java Diagram.getSource versus each freshly built CLI --yaml -pipe meta.source)',
  '/tmp/rustuml-tim-checker/after-probe compare <preserved maker SVG> <fresh Java SVG> (all nine maker pairs)',
  'cargo test --release --test harness_guards (recorded environment, rerun after writing the rejection record; 4 pass/2 fail; record correctly remains non-accepted)'
 ],
 'results':{'parser_unit':{'passed':564,'failed':0},'cli_pipe':{'passed':13,'failed':0},'main_product':{'before_pass':11181,'after_pass':11182,'eligible':11251,'suite_exit':101,'reason':'unlocked baseline improvements; no baseline changed'},'preprocessing_product':{'before_pass':285,'after_pass':286,'eligible':307},'assisted':{'before_pass':10497,'after_pass':10497,'eligible':11251,'failed':754,'skipped':1299,'corpus_pairs':12550,'suite_exit':101},'guards':{'before_pass':4,'after_pass':4,'total':6,'failed':['decimal_literal_ratchet','parity_review_records_are_complete']},'fresh_shipped_cli':{'before_pass':6,'after_pass':15,'total':33,'valid_references':32,'invalid_references':1,'checker_only':{'before_pass':4,'after_pass':8,'total':24,'valid':23},'eight_fresh_maker':{'before_pass':2,'after_pass':6,'total':8},'additional_maker_corpus_diagnostic':{'before_pass':0,'after_pass':1,'total':1},'new_regressions':['checker_sprite_identity']},'exact_cli_metadata_source':{'before_equal':2,'after_equal':22,'valid_reference_total':32,'all_including_invalid_before_equal':3,'all_including_invalid_after_equal':22,'all_total':33},'isolated_heldouts':isolated,'builtin_perturbations':{'before_sources':874,'after_sources':907,'before_panic_failures':379,'after_panic_failures':411,'before_collateral_nonreentrant_panics':378,'after_collateral_nonreentrant_panics':410,'before_invalid_references':7,'after_invalid_references':8,'suite_exit':101,'note':'Aggregate pass counts are contaminated by shared-registry panic cascade; do not use as isolated parity.'}},
 'findings':findings,'perturbations':rows,'verdict':'rejected',
 'repair_dependencies':[
  'TIM owner: preserve actual sprite/plain source in identity; consume dump diagnostics; evaluate theme selectors; execute user subblocks; port Java local/global scope and guarded-variable lookup; preserve comment/blank-line semantics and source locations.',
  'Identity architecture: derive identity and parser representation from one evaluated event stream, or prove equivalent state/environment capture. Re-evaluating a distinct TIM context is not by itself a proof.',
  'Class/style owner: resolve current spot style for circled-character palette across repeated themes; failure in both orders remains a dependency.',
  'Rendering owners: handwritten class polygons and sequence continuation output remain failing despite exact final identity.',
  'Sequence/registry owner: fix unwind cleanup separately; retain the two real self-activation CLI panics as failures. No work on that branch was awaited or incorporated.',
  'A fresh independent checker must re-run the retained failed inputs after repairs. This rejection cannot be relabeled accepted based on narrower expansion statistics.'
 ],
 'caveats':[
  'Java -preproc file mode resolves includes relative to its source location; the two include cases were rerun using pipe mode from repository root. Java SourceProbe and CLI metadata checks disambiguate the actual source string after UmlSource processing.',
  'The generic preprocess_full adapter is diagnostic only and differs from the sequence CLI path; product results come from separate CLI processes.',
  'No forced clock-boundary or concurrent include-file mutation experiment was performed; these remain audited risks, not observed race claims.',
  'The preserved year-only live-date reference is tied to the recorded 2026 environment. The fixed-timestamp counterexample is independent of current year in Java and exposes Rust ignoring that timestamp.',
  'No parent worktree, production code, baselines, comparators, guards, exclusions, references, bullseye targets, earlier evidence or remote was modified by this checker beyond the explicitly requested candidate cherry-picks.'
 ]
}
(record/'review.json').write_text(json.dumps(review,indent=2)+'\n')
print('Wrote rejected review,',len(rows),'pairs')
