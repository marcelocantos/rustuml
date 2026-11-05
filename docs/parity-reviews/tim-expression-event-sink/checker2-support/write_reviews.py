# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json,hashlib,shutil,subprocess
from pathlib import Path
S=Path('/tmp/rustuml-tim-check-2fa74908');SUP=Path('docs/parity-reviews/tim-expression-event-sink/checker2-support')
def load(p):return json.loads(p.read_text())
def dump(p,d):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(d,indent=2)+'\n')
rows=load(SUP/'candidate-additions.json');isolated={r['source']:r for r in load(SUP/'candidate-isolated.json')};sources={r['source']:r for r in load(SUP/'retained-source-replay.json')}
sha=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip();parent='29642ec630e9902c892162a7081cd8a5e1113dac';java='71806a23780b04a5ccde2f8ceb5121edad5eb711'
commands=['CARGO_TARGET_DIR=/tmp/rustuml-tim-check-2fa74908/target CARGO_BUILD_JOBS=4 cargo check --workspace','CARGO_TARGET_DIR=/tmp/rustuml-tim-check-2fa74908/target CARGO_BUILD_JOBS=4 RAYON_NUM_THREADS=4 PLANTUML_STDLIB=/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100 cargo test --release --no-fail-fast --test golden_no_oracle --test golden_pairs --test parity_perturbations --test harness_guards --test pipe_mode','CARGO_TARGET_DIR=/tmp/rustuml-tim-check-2fa74908/target CARGO_BUILD_JOBS=4 cargo test --release -p rustuml-parser --lib','uv run --with pyyaml python /tmp/rustuml-tim-check-2fa74908/checker.py parent isolated','uv run --with pyyaml python /tmp/rustuml-tim-check-2fa74908/checker.py candidate isolated','uv run --with pyyaml python /tmp/rustuml-tim-check-2fa74908/checker.py candidate fresh','uv run --with pyyaml python /tmp/rustuml-tim-check-2fa74908/diagnostics.py','uv run --with pyyaml python /tmp/rustuml-tim-check-2fa74908/final_controls.py','/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java -Djava.awt.headless=true -Duser.timezone=GMT+11:00 -jar /Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar -pipe -tsvg < SOURCE.puml','/tmp/rustuml-tim-check-2fa74908/candidate-rustuml -pipe < SOURCE.puml','/tmp/rustuml-tim-check-2fa74908/probe compare SOURCE.svg RUST.svg','CARGO_TARGET_DIR=/tmp/rustuml-tim-check-2fa74908/target CARGO_BUILD_JOBS=4 cargo clippy --workspace -- -D warnings','cargo fmt --all --check','rustfmt --edition 2024 --config skip_children=true --check crates/rustuml-parser/src/parse/mod.rs crates/rustuml-parser/src/preprocess/mod.rs crates/rustuml-parser/src/preprocess/debug.rs crates/rustuml-parser/src/preprocess/date_format.rs crates/rustuml/tests/pipe_mode.rs']
f={
 'tim-evaluated-event-stream':[
 'REJECTED. Raw-reader policy is now authorized and independently verified; the historical pending-owner statement remains intact in review.json as historical evidence, not a current blocker. Six new raw pipe/file/CLI comparisons and the exact assertion origins 5/8/9 pass. Frozen GET main corpus is unchanged.',
 'checker2_return_plain_nested exposes an extra Phantom class: Java TContext.executeOneLineNotSafe calls simulatePlain for RETURN_FUNCTION, evaluating nested effects but discarding plain text. Rust FunctionDef loses procedure/return kind and call_function emits all plain body events. Source and shipped SVG fail.',
 'checker2_sub_ignored_prelude is Java-valid: Sub.fromFile selects raw located lines without evaluating an unselected prelude. Rust try_includesub executes the whole file in a fresh temp_ctx; an invalid prelude stops before the selected sub is discovered and temp_ctx.error is discarded. Selected class vanishes without CLI failure.',
 'All 40 valid previous TIM checker streams and all 33 original theme streams (one invalid reference) match Java. Previous valid repeated palettes, empty themed class, note, activity, handwritten and continued sequence failures remain failed, not waived by source equality. New return/boolean/type cases falsify generalization.'
 ],
 'tim-expression-event-sink':[
 'REJECTED. Java TContext distinguishes simulatePlain for return functions from addPlain for procedures. Rust FunctionDef records params/body without kind; checker2_return_plain_nested emits an unwanted class despite executing the nested tick once.',
 'Java ReversePolishInterpretor evaluates operand tokens before applying the logical operator. Rust eval_bool uses host ||/&& short circuit: checker2_logical_effects prints calls 0 versus Java 1. checker2_short_circuit_error exits Rust 0 with SVG versus Java error 200.',
 'checker2_literal_re_evaluation executes a quoted $tick() in Rust after eval_bool recursively interprets Value::Str; Java leaves quoted expression literal and renders calls 0. Replacement strings are not always execution-free.',
 'Positive evidence: log/theme/sprite, inline procedure suffix, selector-from order and default side-effect ordering pass. Java deliberately appends the literal inline-procedure suffix without reevaluation; the candidate matches that control. Repeated-theme palette still fails and is not a waiver.',
 'Fatal errors discard the current statement events and stop later statements, but evaluation inside a compound expression continues. checker2_two_errors overwrites the first Bad date pattern with a later overflow diagnostic. Do not call this first-error preservation.'
 ],
 'tim-procedure-global-fork-memory':[
 'REJECTED for complete parameter-memory parity. Prior six checker controls now pass, including declaration-time capture; fresh sibling-frame/global mutation and declaration side-effect order pass.',
 'checker2_literal_default_type and checker2_captured_string_type print 9 instead of Java 0072. Eater.eatDeclareFunction and TFunctionImpl.getNewMemory preserve TValue type; Rust call_function stores to_display strings and eval_expr_to_value reconstructs numeric-looking values with from_str_auto. Captured value type is lost.',
 'checker2_named_default expects first / named but Rust treats $b="named" as positional expression text. Java TFunctionImpl.getNewMemory binds named arguments before positional/default values; Rust call_function enumerates positions only. This is a relevant pre-existing dependency, not an assertion that the correction newly introduced named-argument omission.'
 ],
 'tim-declaration-default-values':[
 'REJECTED. Declaration timing and left-to-right default side effects pass, but a default is not preserved as a typed value through binding. Literal "007" becomes integer 7 and +2 becomes 9, while Java yields string 0072 (checker2_literal_default_type).',
 'Java Eater.eatDeclareFunction stores TValue; TFunctionImpl.getNewMemory binds it without stringification. Rust FuncParam.default is a Value, but call_function immediately calls to_display and LocalMemory stores strings. Later variable reads call from_str_auto.',
 'Named omitted-default case also fails (checker2_named_default). Repair typed memory/default binding and argument selection without changing the independently passing declaration-time evaluation order. Sibling memory is a dependency, not an acceptance waiver.'
 ],
 'tim-literal-variable-lookup':[
 'REJECTED for complete expression-value parity. Both previous quoted/computed-name failures and all prior lookup controls now match Java source and SVG. New Unicode/nested-name source handling passes.',
 'checker2_numeric_string_lookup expects Java 0072 but Rust emits 9: Java VariableExists/GetVariableValue return typed TValue, while Rust eval_builtin_with_expr_args returns String and eval_expr_to_value calls Value::from_str_auto. Correct names do not establish correct return value types.',
 'checker2_unicode_opposite_quotes has exact evaluated source but still fails SVG through downstream quoted declaration rendering. That dependency remains a failure; no comparator or quoting normalization was applied.'
 ],
 'tim-expression-quoted-tokens':[
 'REJECTED. New Unicode/opposite quote/nested getter/literal function-text controls establish bounded token-scanning progress. Numeric-looking string lookup loses its type, yielding 9 instead of Java 0072. Quoted boolean function-like text is re-evaluated by eval_bool, producing calls 1 instead of 0.',
 'Java TokenType.eatOneToken and TValue distinguish quoted tokens from numbers throughout evaluation. Rust reconstructs Values from display strings at variable/builtin boundaries. Repair the value representation and boolean evaluator, not fixture labels or string special cases.',
 'The opposite-quote case has exact source but unequal SVG; downstream class parsing remains a dependency and is not waived.'
 ],
 'tim-explicit-date-instant':[
 'REJECTED. Signed literal extrema, fixed offsets, prior valid date cases and current width/quote boundary controls pass. Java-valid arithmetic overflow does not: checker2_sum_wrap and checker2_difference_wrap produce 1901/2038 dates in Java but Rust exits 1 with no SVG.',
 'Java TValue.plus/minus operate on int fields, preserving Java signed-32 wrap before DateFunction multiplies to long milliseconds. Rust arithmetic uses f64/number_value and checks the unwrapped result with parse::<i32>(). Do not classify these valid expressions as invalid overflow controls.',
 'Named-host zzzz is still wrong: Java SimpleDateFormat.subFormat selects TimeZone.LONG at width >=4; Rust date_format::format always uses tz.name (abbreviation). checker2_long_zone source and SVG fail.',
 'Direct invalid pattern, literal overflow and selected function/include/default errors now cause CLI failure without SVG. Negative references remain failures in the unchanged SVG suite, separately reported as successful rejection controls.'
 ],
 'tim-date-pattern-tokens':[
 'REJECTED. Valid width/quote/negative instant/fractional-zone probes pass, but checker2_long_zone emits AEST twice rather than the long Australian Eastern Standard Time name. JDK21 SimpleDateFormat.subFormat selects SHORT/LONG based on token run length; Rust ignores z width.',
 'checker2_sum_wrap/difference_wrap are valid Java arithmetic followed by date formatting, not malformed literals. Java wraps int arithmetic; Rust produces a larger number then rejects it. Both new valid references are preserved.',
 'Invalid XXXX, Q, unmatched quote, upper/lower literal overflow, function/default/include controls exercise typed errors and actual CLI. Rejection successes are separately classified, never counted as valid SVG matches. checker2_short_circuit_error still succeeds because Rust omits Java-required operand evaluation.',
 'A later error overwrites the first error in checker2_two_errors. Preserve first failure and stop expression evaluation once fatal. No new formatter numeric constant was found lacking the named calendar/JDK provenance; the failures are missing semantics, not tolerances.'
 ],
 'tim-parser-error-propagation':[
 'REJECTED as a complete end-to-end error packet; the two new parse_with_base handoffs themselves are corroborated. Both error.take checks return ParseError before consuming partial lines. Actual CLI rejects all four retained root controls plus new pattern/overflow errors inside functions/defaults/includes/selected subblocks and selected block 1, without SVG.',
 'checker2_short_circuit_error is Java-invalid but Rust exits 0 with SVG. Its typed error is None because eval_bool short-circuits, so this is an upstream evaluator defect, not evidence that error.take ignores Some. checker2_two_errors returns the later overflow diagnostic instead of Java first Bad date pattern.',
 'checker2_sub_ignored_prelude is Java-valid but drops its selected class: try_includesub evaluates the whole external file in temp_ctx, stops on an irrelevant prelude error and discards that error and the undiscovered selected sub. Java Sub.fromFile selects raw lines first.',
 'Default CLI block 0 versus Java pipe all-block behavior is separately classified. Rust --block=1 on checker2_multi_block_error exits 1 with no SVG; the default block-0 success is not labeled a new error-propagation bug.',
 'Both preprocessing calls inspected. All deterministic fatal controls trigger the first call; no second-pass-only dynamic error was demonstrated. The two calls differ in preserve_teoz_pragma (and source projection), not a separate tested failure contract. No test injection or claim of branch coverage was substituted.',
 'Public preprocess_full uses different body-origin marking from parse_with_base: function diagnostic reports caller line 5 through the convenience API but body line 2 through parse/CLI, matching Java. This distinction is recorded, not treated as proof that every convenience API propagates errors.'
 ]}
f['tim-theme-post-expansion-source-identity']=f['tim-evaluated-event-stream']+['The older theme mechanism remains rejected: its retained repeated palette, handwritten, continuation and new general event-stream counterexamples remain executable. This is a new review, not an edit to the earlier reviewer record.']
links={
 'tim-theme-post-expansion-source-identity':['tim-evaluated-event-stream','tim-expression-event-sink','tim-parser-error-propagation'],
 'tim-evaluated-event-stream':['tim-expression-event-sink','tim-parser-error-propagation'],
 'tim-expression-event-sink':['tim-evaluated-event-stream','tim-parser-error-propagation'],
 'tim-procedure-global-fork-memory':['tim-declaration-default-values'],
 'tim-declaration-default-values':['tim-procedure-global-fork-memory'],
 'tim-literal-variable-lookup':['tim-expression-quoted-tokens'],
 'tim-expression-quoted-tokens':['tim-literal-variable-lookup','tim-expression-event-sink'],
 'tim-explicit-date-instant':['tim-date-pattern-tokens','tim-parser-error-propagation'],
 'tim-date-pattern-tokens':['tim-explicit-date-instant','tim-parser-error-propagation'],
 'tim-parser-error-propagation':['tim-date-pattern-tokens']}
for m,findings in f.items():
 base=Path('docs/parity-reviews')/m;old=base/'review.json'
 if old.exists():
  original=load(old);account=original['account_commit'];impl=original['implementation_commit'];dest=base/'review-2.json'
 else:
  original={};account='0530072a62919c4008278e84e5def3fbd11ec098' if m=='tim-parser-error-propagation' else '67e53172fa470cdf2cec61c5aa0323dad11ba224';impl='b384bc315f31b890abb3cbd91197b6f6666ca1ad' if m=='tim-parser-error-propagation' else 'c1730d656949364dd12f0fa3bcc69b1dc04b1941';dest=old
 pert=[]
 # Every earlier checker failure for this mechanism remains represented at its original path.
 for prev in original.get('perturbations',[]):
  p=prev['source'];r=isolated.get(p)
  if r:
   pert.append(dict(source=p,golden=str(Path(p).with_suffix('.svg')),axes=prev.get('axes',['retained earlier checker','exact candidate replay']),result='pass' if r['svg_match'] else 'fail',cli_exit=r['cli_exit'],comparison_exit=r['comparison_exit'],kind='retained previous review',source_equal=sources.get(p,{}).get('source_equal')))
 for r in rows:
  if r['mechanism'] not in [m,*links[m]]:continue
  pert.append(dict(source=r['source'],golden=str(Path(r['source']).with_suffix('.svg')),axes=r['axes'],result='pass' if r['svg_match'] else 'fail',kind=r['kind'],java_exit=r['java_exit'],cli_exit=r['cli_exit'],source_equal=r['source_equal'],reference_classification='valid raw pipe' if r['java_valid'] else 'negative Java output; never a positive SVG',negative_rejection=r['negative_rejection']))
 # Duplicate paths are not separate evidence.
 pert=list({r['source']:r for r in pert}.values())
 record=dict(schema_version=1,mechanism=m,reviewer=dict(task_id='t3-2fa74908-independent-corrected-TIM-checker',model='gpt-6-astra / high / inherited Codex intentional; fresh independent checker; no children'),account_commit=account,implementation_commit=impl,java_revision=java,commands=commands,perturbations=pert,findings=findings,verdict='rejected',combined_tested_commit=sha,pre_policy_tested_commit='86e0d0953acc1bbc660304265033c3314fa31150',parent_commit=parent,original_candidate='b384bc315f31b890abb3cbd91197b6f6666ca1ad',preparation_mappings='docs/parity-reviews/tim-expression-event-sink/checker2-support/cherry-pick-mappings.json',followup_commits=['c1730d656949364dd12f0fa3bcc69b1dc04b1941','b384bc315f31b890abb3cbd91197b6f6666ca1ad','b62d8ce967b6dda7d85bb6dc28ff92509f6066f9','136ee057e0ffc3347ecdfafc9cf46e0bafa50197'],java_jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',reference_contract='Owner-selected raw Java pipe/file. Frozen historical GET corpus retained and separately reported. Negative evidence unchanged.',evidence='docs/parity-reviews/tim-expression-event-sink/checker2-support/README.md',independently_accepted_submechanisms=[],global_acceptance='rejected; green bounded invariants do not certify the packet')
 record['dependency_perturbations']=[r for r in pert if not r['source'].startswith('test-diagrams/perturbations/'+m+'/')]
 record['perturbations']=[r for r in pert if r['source'].startswith('test-diagrams/perturbations/'+m+'/')]
 dump(dest,record)
shutil.copy2(S/'cherry-pick-mappings.json',SUP/'cherry-pick-mappings.json')
# Final replay manifest covers all generated diagram/helper inputs; do not regenerate old references.
manifest=[{k:r[k] for k in ['source','mechanism','kind','axes','zone','debug','original_source','reference_contract'] if k in r} for r in rows];dump(SUP/'final-manifest.json',manifest)
print('wrote',len(f),'reviews and',len(rows),'additions',sha)
