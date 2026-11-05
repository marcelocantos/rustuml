# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Author new independent records; never overwrite an existing review."""
import json
from pathlib import Path
S=Path(__file__).resolve().parent;ROOT=S.parents[3];BASE=ROOT/'docs/parity-reviews'
def load(p):return json.loads(p.read_text())
def dump(p,d):p.write_text(json.dumps(d,indent=2)+'\n')
prep=load(S/'preparation.json'); frozen=load(S/'after-frozen.json');index={r['source']:r for r in frozen}
new=[]
for suite in ['fresh','retained','supplement']:
 refs={r['source']:r for r in load(S/('reference-'+suite+'.json'))}
 for row in load(S/('after-'+suite+'.json')):new.append(dict(refs[row['source']],**row))
findings={
 'tim-function-execution-kind':[
 'REJECTED. Recursive return plain suppression, nested procedure emission and include execution-kind restoration now match source and SVG. However checker3_loop_early_return yields Java 1 / 1 versus Rust 3 / 3; checker3_foreach_return yields Java 0052 / 1 versus Rust 0062 / 2. TContext.executeLines returns immediately when executeOneLineSafe yields a TValue. Rust try_while/try_foreach continue buffered body iteration and further iterations after return_signal is set, overwriting the first return and executing later mutations.',
 'checker3_procedure_expression is rejected by Java TFunctionImpl.executeReturnFunction (illegal procedure call in value context), but Rust eval_function_call accepts it and emits Unexpected. This is distinct from the explicitly retained no-return !function extension, which is also Java-negative and is not positive parity evidence.',
 'Bounded guidance: propagate return control flow through loop interpreters, and enforce call-context function kind. Keep the demonstrated simulatePlain/addPlain distinction and nested execution-kind save/restore. No production fix is included.'
 ],
 'tim-typed-values-and-binding':[
 'REJECTED, including a fresh lost pass: checker3_substr_json passes before but fails after. Java Substr uses TValue.toInt, whose JSON/string value is zero; it renders cedar. Rust eval_builtin_with_expr_args stringifies the new Json(2), then eval_one_builtin_args reparses it as offset2, rendering dar. The prior pass used the unimplemented str2json empty-string fallback, so it was accidental but remains a real measured lost pass.',
 'checker3_substr_string and checker3_hex_string independently expose the same typed-argument leak. checker3_hex_negative uses i64 hex formatting (fffffffffffffffd) instead of Java Integer.toHexString (fffffffd). checker3_unicode_index uses UTF8 byte indices (6 instead of Java UTF16 3); checker3_unicode_slice uses Unicode scalar indices (b instead of a).',
 'checker3_snapshot renders 15 / 5 rather than Java 14 / 5. ShuntingYard resolves variable tokens through Knowledge before ReversePolishInterpretor executes calls. Rust recursively reads the right operand after its left function has mutated memory. checker3_snapshot_logical exercises the independent logical axis of the same divergence.',
 'checker3_json_index has typed JSON memory but no Java TContext.asKnowledge/fromJson path for member/index access; fallback emits the entire object followed by .entry[0]2, rather than 0092. checker3_literal_macro reprocesses literal returned NEXT through the legacy token substitution loop, yielding changed instead of Java NEXT.',
 'Positive evidence: typed declaration capture, named/default binding, sibling-frame parameters, boolean/JSON maker controls, signed wrap/division and date conversion. Repair typed argument contracts per builtin, stage expression token resolution as Java does, and preserve literal expansion boundaries. Floats/no-return extensions and Java-invalid shift/unary forms are not advertised as full grammar support.'
 ],
 'tim-eager-operands-first-error':[
 'REJECTED. Recursive AND/OR eager effects and Unicode quoted-function truth controls pass source and SVG. Guards preserve the first recorded date error in checker3_fatal_nested and retained maker argument/logical controls. This is bounded positive evidence, not all-error parity.',
 'checker3_snapshot_logical gives Java 0 / 1 versus Rust 1 / 1: Java ShuntingYard freezes the variable token before executing $set(), whereas Rust reads the right operand after mutation. Eager calls do not imply identical operand-resolution timing.',
 'Earlier errors can still be missing entirely: Substr(-1) throws before the later bad date in Java, but Rust coerces negative index to default0 and reports the later date. Unknown %missing_checker3 similarly raises Java Unknown built-in function, but Rust returns empty and later reports X pattern failure. The isolated only variants are Java-negative while Rust exits0 with SVG including After. This is swallowed first failure and continued execution, not overwriting an already-set ctx.error.',
 'Repair error production and typed builtin contracts as well as cancellation. Existing fail() first-writer behavior and parse error.take handoffs are corroborated and should not be weakened.'
 ],
 'tim-external-sub-selection':[
 'REJECTED. Repeated selected blocks, nested raw start/end behavior, ignored fatal prelude/tail and caller-relative nested include controls pass source and SVG. Selected-error control fails with the first Q diagnostic at included source line1; no later X replacement.',
 'Java-valid checker3_tabs uses !startsub followed by a tab; TLineType/EaterStartsub accepts whitespace, Rust strip_prefix("!startsub ") does not, silently dropping the selected mutation (0 versus3). checker3_dynamic_selector is evaluated by Java EaterIncludesub.applyFunctionsAndVariables; Rust regex matches before expansion and leaves the directive as plain source (0 versus3).',
 'Retained maker sub_caller_relative has identical evaluated source and Child source_line0 in the Rust model, but unequal SVG: render/class.rs substitutes entity_index+1 for zero, and entity emission ordering differs. This remains a failure and is not a source-only waiver. No renderer production was imported or fixed.',
 'Keep raw selection before execution, but use the actual directive/selector model and preserve included origins. The account wording that unselected contents cannot fail is too broad: Java Sub.fromFile still parses every STARTSUB marker via EaterStartsub; body execution is what selection suppresses.'
 ],
 'tim-zone-display-width':[
 'No fresh semantic falsifier of SHORT/LONG name selection was found in the declared environment. All eight new width/zone cases and all three retained maker zone cases match actual CLI SVG and exact source with matching TZ. Axes include London standard/daylight, Lord Howe fractional daylight, Dublin winter, Kathmandu historical offset, Casablanca Ramadan/summer, UTC and a negative fixed fractional offset.',
 'Independent remote rerun of ZoneNames.java enumerated631 IDs and reproduced zone_names.json byte-for-byte (SHA256 1b99a8affbdcaba1600e0f18d39126e68a461b1c3040ffc9eed29606d7f87483). This is TimeZone API locale data, not diagram output or fixture-shaped lookup. JDK SimpleDateFormat width<4 versus>=4 selection and the Rust data flow agree on the measured paths.',
 'REJECTED for acceptance of this packet, not asserted to be a newly demonstrated zone defect. The required standing no-oracle perturbation gate is red and cannot replay declared named/fractional environments using its fixed GOLDEN_DEBUG. This checker is not authorized to change that gate or acceptance contract. The positive local invariant is explicitly characterized; universal runtime/OS/locale/zone-rule equivalence, unknown-zone fallback and non-Unix behavior remain unproved. No environment mismatch is counted as a new product regression.'
 ],
 'tim-explicit-date-instant':[
 'Fresh signed multiplication wrap, INT_MIN/-1 division and negative truncating quotient followed by date conversion all match Java source and shipped SVG. The previous checker2_sum_wrap and checker2_difference_wrap valid CLI errors are repaired; no new valid CLI errors occur on the frozen1044 cohort.',
 'The root test-only correction is sound: exact unchanged class and sequence assertion sources cause remote Java NumberFormatException in TValue.fromNumber/Integer.parseInt before DateFunction. Rust emits Integer literal outside signed 32-bit range at line2. All582 parser tests pass. Java does not emit that exact Rust diagnostic text; its literal exception escapes, so the claim is causal equivalence and fatal rejection, not identical wording.',
 'The complete inherited mechanism remains REJECTED with preserved negative references and unresolved upstream expression/builtin semantics. Supported invariant: signed Java i32 add/subtract/multiply/truncating division before TValue.toInt date conversion on the exercised valid expressions. Floats, shifts and unary variable expressions are not inferred as accepted Java grammar.'
 ],
 'tim-parser-error-propagation':[
 'The two parse_with_base error.take handoffs remain correct on checked inputs. Exact class/sequence pattern and literal controls all reject with zero SVG; literal errors report line2. Selected include and nested function errors preserve the demonstrated source origins. All582 parser tests and15 pipe tests pass.',
 'Complete packet REJECTED: checker3_fatal_substr_only and checker3_fatal_unknown_only exit0 and render After despite Java failure. Their paired compound/default controls report a later date instead of the earlier Java exception/unknown function. Upstream error creation is missing; this is not a failure of error.take on Some.',
 'Second-pass-only dynamic failure was not demonstrated. Convenience Vec-returning preprocessor APIs still discard errors by design. Multi-block default block0 versus explicitly selected block1 remains a separate CLI contract; no change or waiver is introduced.'
 ],
}
legacy={
 'tim-expression-event-sink': ['tim-function-execution-kind','tim-eager-operands-first-error'],
 'tim-evaluated-event-stream':['tim-function-execution-kind','tim-external-sub-selection'],
 'tim-theme-post-expansion-source-identity':['tim-function-execution-kind','tim-external-sub-selection'],
 'tim-procedure-global-fork-memory':['tim-typed-values-and-binding'],
 'tim-declaration-default-values':['tim-typed-values-and-binding'],
 'tim-literal-variable-lookup':['tim-typed-values-and-binding'],
 'tim-expression-quoted-tokens':['tim-typed-values-and-binding','tim-eager-operands-first-error'],
 'tim-date-pattern-tokens':['tim-explicit-date-instant','tim-zone-display-width','tim-eager-operands-first-error'],
}
for m,deps in legacy.items():
 findings[m]=['REJECTED as the complete inherited mechanism. The new typed execution changes repair the prior measured cases, but fresh independent counterexamples in '+', '.join(deps)+' remain relevant dependencies; source equality never waives unequal SVG.',
 'Reassessment includes all own preserved cases in the frozen1449 actual CLI replay plus the source-guided fresh tests linked below. Old reviews and every old input/reference remain byte-identical. Five class TIM handoff controls and six raw reader controls independently match Java source and SVG; no reader-policy re-investigation or change was made.',
 'The historical repeated theme palette, opposite-quote aliases, empty themed canvas/note, handwritten, activity and sequence failures remain failed where present. Newly corrected source/event behavior does not certify their larger end-to-end clusters.']

def entry(r):
 return {k:v for k,v in dict(source=r['source'],golden=str(Path(r['source']).with_suffix('.svg')),axes=r.get('axes',['preserved regression input','candidate replay']),result='pass' if r['svg_match'] else 'fail',kind=r.get('kind','frozen prior checker'),cli_exit=r['cli_exit'],reference_invalid=r['reference_invalid'],source_equal=r.get('source_equal'),java_exit=r.get('java_exit'),reference_contract='raw pipe for new cases; existing provenance unchanged',debug=r.get('debug','date=1774210426000,tz=AEDT+1100'),zone=r.get('zone','GMT+11:00'),cwd=r.get('cwd','.')).items() if v is not None}
commands=[
 'ssh -T -o BatchMode=yes marcelo@192.168.1.112 /Users/marcelo/rustuml-reference/bin/new-worker',
 "ssh -T -o BatchMode=yes marcelo@192.168.1.112 '/Users/marcelo/rustuml-reference/bin/reference --cwd /Users/marcelo/rustuml-reference/scratch/worker.l4Yx4xIn --timezone GMT+11:00 -- -pipe -tsvg' < SOURCE.puml",
 'uv run --with pyyaml python docs/parity-reviews/tim-typed-values-and-binding/checker3-support/check.py before frozen',
 'uv run --with pyyaml python docs/parity-reviews/tim-typed-values-and-binding/checker3-support/check.py after frozen',
 'uv run --with pyyaml python docs/parity-reviews/tim-typed-values-and-binding/checker3-support/check.py reference fresh (also retained, supplement; each row records zone/cwd)',
 'uv run --with pyyaml python docs/parity-reviews/tim-typed-values-and-binding/checker3-support/check.py after fresh (also before and retained/supplement)',
 'CARGO_TARGET_DIR=/tmp/rustuml-tim-checker3.dRQvKH/target CARGO_BUILD_JOBS=4 RAYON_NUM_THREADS=4 PLANTUML_STDLIB=/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib RUSTUML_DEBUG=date=1774210426000,tz=AEDT+1100 cargo test --release --no-fail-fast --test golden_no_oracle --test golden_pairs -- --nocapture',
 'CARGO_TARGET_DIR=/tmp/rustuml-tim-checker3.dRQvKH/target CARGO_BUILD_JOBS=4 cargo test --release -p rustuml-parser --lib',
 'CARGO_TARGET_DIR=/tmp/rustuml-tim-checker3.dRQvKH/target CARGO_BUILD_JOBS=4 cargo test --release -p rustuml --bin rustuml --test pipe_mode',
 'CARGO_TARGET_DIR=/tmp/rustuml-tim-checker3.dRQvKH/target CARGO_BUILD_JOBS=4 cargo clippy --workspace -- -D warnings',
 'rustfmt --edition 2024 --config skip_children=true --check crates/rustuml-parser/src/preprocess/mod.rs crates/rustuml-parser/src/preprocess/debug.rs crates/rustuml-parser/src/preprocess/date_format.rs crates/rustuml-parser/src/parse/mod.rs crates/rustuml/tests/pipe_mode.rs'
]
for m,fs in findings.items():
 directory=BASE/m
 reviews=sorted(directory.glob('review*.json'),key=lambda p:1 if p.name=='review.json' else int(p.stem.split('-')[-1]))
 old=load(reviews[-1]) if reviews else None
 dest=directory/(f'review-{(1 if reviews[-1].name=="review.json" else int(reviews[-1].stem.split("-")[-1]))+1}.json' if reviews else 'review.json')
 own=[entry(r) for r in new if r['mechanism']==m]
 if old:
  prior=[dict(index[r['source']],axes=r.get('axes',['retained'])) for r in old['perturbations'] if r['source'] in index]
  # Entire own prior cohort is replayed, regardless of past verdict/result.
  own += [entry(r) for r in prior if r['source'] not in {e['source'] for e in own}]
 dependencies=legacy.get(m,[])
 if not dependencies:
  dependencies={'tim-explicit-date-instant':['tim-typed-values-and-binding','tim-zone-display-width'],'tim-parser-error-propagation':['tim-eager-operands-first-error','tim-external-sub-selection'],'tim-function-execution-kind':['tim-eager-operands-first-error'],'tim-typed-values-and-binding':['tim-eager-operands-first-error'],'tim-eager-operands-first-error':['tim-typed-values-and-binding']}.get(m,[])
 record=dict(schema_version=1,mechanism=m,reviewer=dict(task_id='t3-bf706fe0-fresh-independent-typed-TIM-checker',model='gpt-6-astra / high / Codex in T3 Code; no children; did not implement candidate'),account_commit=old['account_commit'] if old else '819485f4001a94056a192e4e8688729d4c9de3e8',implementation_commit='0dd2de894fea40d5e9ed81efb6a54d837f80fa4c',java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',java_jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',prepared_before=prep['before'],combined_tested_commit=prep['after'],parent_commit=prep['parent'],root_test_only_followup='6f52f87e532d549b047f7ccd3fe7fb577fc56df9',commands=commands,perturbations=own,dependency_perturbations=[entry(r) for r in new if r['mechanism'] in dependencies],findings=fs,verdict='rejected',packet_verdict='rejected; no production changes accepted or integrated by this review',evidence='docs/parity-reviews/tim-typed-values-and-binding/checker3-support/README.md',environment='All Java remote Monkey Man: OpenJDK21.0.5, Graphviz12.2.0, en_AU, UTF8, headless wrapper; origin21.0.13/16.1.0 equivalence unproven. Local Rust and unchanged strict comparator; declared per-input timezone/cwd.',scope='Evidence only; original accounts/reviews/reference data/frozen rules/targets unchanged. No class or sequence candidate production imported.')
 if old:record['supersedes_without_overwriting']=str(reviews[-1].relative_to(ROOT))
 if dest.exists():raise RuntimeError('Refusing to overwrite '+str(dest))
 dump(dest,record)
 print(dest.relative_to(ROOT))
dump(S/'final-manifest.json',new)
