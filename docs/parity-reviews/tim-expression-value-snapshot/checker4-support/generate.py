# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Source-guided independent checker inputs; never modifies a reference."""
import hashlib, json, shutil
from pathlib import Path
ROOT=Path('/Users/marcelo/.t3/worktrees/rustuml/t3-cb14b5f3')
S=Path('/Users/marcelo/Library/Caches/rustuml-review-cb14b5f3')
STAGE=S/'stage'
rows=[]
def add(mechanism,name,body,axes,provenance,env=None,negative=False):
    path=Path('test-diagrams/perturbations')/mechanism/('checker4_'+name+'.puml')
    p=STAGE/path;p.parent.mkdir(parents=True,exist_ok=True)
    p.write_text('@startuml\n'+body.rstrip()+'\n@enduml\n')
    rows.append(dict(id=name,mechanism=mechanism,source=str(path),cwd=str(path.parent) if 'sub-directive' in mechanism else '.',zone='GMT+11:00',debug='date=1774210426000,tz=AEDT+1100',env=env or {},axes=axes,provenance=provenance,intended_negative=negative))
B='tim-builtin-typed-contracts';X='tim-expression-value-snapshot';L='tim-literal-substitution-boundary';R='tim-return-unwind';F='tim-foreach-json-binding';S1='tim-sub-directive-selection';G='tim-builtin-registration'
add(B,'typed_default_window','''!function $window($start="1", $size=%str2json("2"))
!return %substr("maple",$start,$size)
!endfunction
!$a=$window()
!$b=%substr("maple",%intval("1"),3)
!$c=%dec2hex(%str2json("-7"))
class "$a / $b / $c" as Window''',['default value kind','nested builtin conversion'],'TValue.toInt; Substr; Dec2hex')
add(B,'surrogate_case_conversion','''!$high=%substr("🦊",0,1)
!$low=%substr("🦊",1,1)
!$a=%strpos(%upper($high)+$low,"🦊")
!$b=%strpos($high+%lower($low),"🦊")
class "$a / $b" as Surrogates''',['surrogate split position','case conversion path'],'Upper/Lower String case conversion; TValue.toString')
add(B,'surrogate_projection','''!$half=%substr("🦊",0,1)
class "left $half right" as Projection''',['isolated UTF16','final text projection'],'Substr; UTF-8 encoder/SVG escaping')
add(B,'chr_typed_numeric','''!$a=%strpos(%chr("66"),"B")
!$b=%strpos(%chr(%str2json("67")),"C")
class "$a / $b" as Character''',['string versus JSON','numeric builtin contract'],'Chr.executeReturnFunction TValue.toInt')
add(B,'strlen_arity_error','''!$v=%strlen("a","b")
!$later=%date("Q",0)
class After''',['arity','first failure'], 'Strlen.canCover; ReversePolishInterpretor.canCover',negative=True)
step='''!$n=4
!function $step()
!$n=$n+1
!return 10
!endfunction
!function $join($a,$b)
!return %string($a)+"/"+%string($b)
!endfunction
'''
add(X,'snapshot_call_context',step+'''!$a=$join($step(),$n)
class "$a / $join($step(),$n) / $n" as Context''',['expression versus plain call','argument mutation'], 'ShuntingYard; EaterFunctionCall.analyze')
add(X,'snapshot_index_effect','''!$n=2
!$data=["013","021"]
!function $index()
!$n=$n+3
!return 0
!endfunction
!function $tick()
!$n=$n+1
!return 8
!endfunction
!$v=$n+$data[$index()]+$tick()+$n
class "$v / $n" as Indexed''',['index preparation effects','left-to-right snapshot order'],'ShuntingYard.getVariable; TContext.fromJson; VariableManager.replaceJson')
add(X,'snapshot_condition',step+'''!$v=($step()>$n)&&($n==4)
!if ($step()>$n)||($n==0)
class "yes / $v / $n" as Conditional
!else
class "no / $v / $n" as Conditional
!endif''',['logical/comparison operators','conditional statement'],'TokenOperator; ShuntingYard; TValue.logicalAnd')
add(X,'json_missing_chain','''!$data={"ok":{"name":"ash"}}
!$v=$data.absent.name
class "[$v]" as Missing''',['missing member','nested member access'],'VariableManager.replaceJson null traversal',negative=True)
add(X,'json_scalar_types','''!$data={"items":["001",2],"flags":{"a":true,"b":null}}
!$idx="1"
!$v=$data.items[$idx]+3
!$w=$data.flags.a+"/"+$data.flags.b
class "$v / $w" as JsonValues''',['JSON numbers versus literals','boolean/null member projection'],'TContext.fromJson; TValue.add')
add(L,'literal_definelong','''!definelong TOKEN
changed
!enddefinelong
!$v="TOKEN"
!function $word()
!return "TOKEN"
!endfunction
class "$v / $word()" as Literal''',['returned versus variable literal','legacy multiline-function registration'],'TContext.applyFunctionsAndVariables; TFunctionImpl.finalizeEnddefinelong')
add(L,'literal_newline_boundary',r'''!define TOKEN amber
!$v="TOKEN"
class "north\nTOKEN / $v" as Boundary''',['backslash-n word boundary','direct versus substituted token'],'VariableManager.justAfterBackslashN/getVarnameAt')
add(L,'literal_define_effects','''!$n=0
!function $next()
!$n=$n+1
!return "LATER"
!endfunction
!define STORED $next()
!define LATER cedar
!$v="STORED"
class "STORED / $v / $n" as Capture''',['declaration-time effect','late definition and literal'],'EaterAffectationDefine.analyze; TContext.applyFunctionsAndVariables')
add(R,'return_sibling_nested','''!$n=0
!function $pick($label)
!while $n<5
!foreach $item in ["04","09"]
!if $item=="04"
!$n=$n+1
!return $label+$item
!endif
!$n=99
!endfor
!endwhile
!return "miss"
!endfunction
!$a=$pick("oak")
!$b=$pick("elm")
class "$a / $b / $n / $item" as Return''',['nested foreach/while/conditional','sibling invocation state restore'],'TContext.executeLines; TFunctionImpl execution memory')
add(R,'return_condition_effect','''!$n=0
!function $condition()
!$n=$n+1
!return 1
!endfunction
!function $choose()
!while $condition()
!return "kept"
!$n=100
!endwhile
!return "miss"
!endfunction
!$v=$choose()
class "$v / $n" as Condition''',['condition side effect','post-return cancellation'],'CodeIteratorWhile; TContext.executeLines')
add(R,'procedure_argument_error','''!procedure $emit($x)
class Unexpected
!endprocedure
!$v=$emit(%substr("x",-1))
!$later=%date("Q",0)
class After''',['argument failure versus procedure-kind failure','later statement cancellation'],'ReversePolishInterpretor; TFunctionImpl.executeReturnFunction',negative=True)
add(F,'foreach_shadow_sibling','''!$item="root"
!function $read()
!return $item+1
!endfunction
!function $walk($item="local")
!$out=""
!foreach $item in ["03",4]
!$out=$out+":"+$item+":"+$read()
!endfor
!return $out
!endfunction
!$v=$walk()
class "$v / $item" as Scope''',['parameter override versus global iteration','sibling visibility and element type'],'CodeIteratorForeach.setLoopVariable; TMemoryLocal')
add(F,'foreach_object_rebind','''!$object={"z":"Z","a":"A","m":"M"}
!$out=""
!foreach $key in $object
!$out=$out+$key
!$object={"new":"N"}
!endfor
class "$out / $key" as ObjectOrder''',['insertion order','collection rebinding during iteration'],'ExecutionContextForeach; JsonObject.names')
add(F,'foreach_nested_same_name','''!$out=""
!foreach $entry in ["a","b"]
!foreach $entry in [1,2]
!$out=$out+$entry
!endfor
!$out=$out+$entry
!endfor
!foreach $entry in []
!$out="unexpected"
!endfor
class "$out / $entry" as Nested''',['nested same binding','empty collection preservation'],'CodeIteratorForeach; ExecutionContextForeach')
add(F,'foreach_scalar_error','''!foreach $item in %str2json("3")
class Never
!endfor
class After''',['JSON scalar kind','error versus empty iteration'],'EaterForeach.size',negative=True)
subpath=STAGE/'test-diagrams/perturbations'/S1/'checker4-resources';subpath.mkdir(parents=True,exist_ok=True)
(subpath/'blocks.inc').write_text('''!$bad=%date("Q",0)
!startsub\tchosen
!$n=$n+2
!endsub
!startsub chosen
!include child.inc
!endsub
''')
(subpath/'child.inc').write_text('!$n=$n+5\n')
(subpath/'bad_tail.inc').write_text('!startsub chosen\n!$n=$n+2\n!endsub\n!startsub broken-name\n!endsub\n')
(subpath/'comment.inc').write_text("/'\n!startsub bad-name\n'/\n!startsub\tchosen\n!$n=$n+3\n!endsub\n")
add(S1,'sub_function_selector','''!$n=0
!function $select()
!return "checker4-resources/blocks.inc!chosen"
!endfunction
!includesub\t$select()
class "n=$n" as Selected''',['function expansion and tabs','repeated blocks/nested relative include'],'EaterIncludesub; Sub.fromFile; TContext.executeIncludesub')
add(S1,'sub_local_end_marker','''!$n=0
!startsub chosen
!$n=$n+2
!endsub ' marker suffix
!$which="chosen"
!includesub $which
class "n=$n" as Local''',['local marker suffix','expanded local selector'],'TLineType.ENDSUB; CodeIteratorSub')
add(S1,'sub_commented_invalid','''!$n=0
!includesub checker4-resources/comment.inc!chosen
class "n=$n" as Comment''',['commented malformed marker','tab whitespace'],'UncommentReadLine; Sub.fromFile')
add(S1,'sub_invalid_after_selected','''!$n=0
!includesub checker4-resources/bad_tail.inc!chosen
!$later=%date("Q",0)
class After''',['malformed marker after selected section','first-error priority'],'Sub.fromFile whole-file validation',negative=True)
add(G,'registry_unknown_text','''!$tag="elm"
class "%fresh_unregistered($tag) / $tag" as Registry''',['unknown plain calls','inner variable substitution'],'TContext.getFunctionNameAt/applyFunctionsAndVariables')
add(G,'registry_unknown_expression','''!$v=%fresh_unregistered(2)
!$later=%date("Q",0)
class After''',['unknown expression calls','first failure priority'],'ReversePolishInterpretor.getFunction',negative=True)
add(G,'registry_filtered_environment','''!$a=%getenv("PLANTUML_CHECKER4")
!$b=%getenv("CHECKER4_PRIVATE")
!$c=%getenv("plantuml.security.checker4")
class "$a / [$b] / [$c]" as Environment''',['allowed versus denied names','preserve numeric-looking text'], 'Getenv; SecurityProfile.canWeReadThisEnvironmentVariable',env={'PLANTUML_CHECKER4':'007,oak','CHECKER4_PRIVATE':'benign-control','plantuml.security.checker4':'benign-hidden','PLANTUML_SECURITY_PROFILE':'LEGACY'})
add(G,'registry_unsecure_environment','''!$a=%getenv("CHECKER4_PRIVATE")
!$b=%getenv("plantuml.security.checker4")
class "$a / [$b]" as Environment''',['UNSECURE profile','security prefix still denied'],'SecurityProfile.init/canWeReadThisEnvironmentVariable',env={'CHECKER4_PRIVATE':'visible-control','plantuml.security.checker4':'benign-hidden','PLANTUML_SECURITY_PROFILE':'UNSECURE'})
add(G,'registry_registered_unimplemented','''class "%boolval(0)" as Registered''',['registered name','plain text dispatch implementation'],'TContext.addStandardFunctions; BoolVal.executeReturnFunction')
shutil.copyfile(ROOT/'docs/parity-reviews/tim-corrective-execution-maker/SourceProbe.java',STAGE/'SourceProbe.java')
for row in rows:
    row['source_sha256']=hashlib.sha256((STAGE/row['source']).read_bytes()).hexdigest()
(S/'fresh-manifest.json').write_text(json.dumps(rows,indent=2)+'\n')
resources=[]
for path in sorted(STAGE.rglob('*')):
    if path.is_file():resources.append(dict(local_path=str(path),remote_relative_path=str(path.relative_to(STAGE)),sha256=hashlib.sha256(path.read_bytes()).hexdigest(),bytes=path.stat().st_size))
request=dict(manifest_path=str(S/'fresh-manifest.json'),manifest_sha256=hashlib.sha256((S/'fresh-manifest.json').read_bytes()).hexdigest(),case_count=len(rows),resources=resources,wrapper='/Users/marcelo/rustuml-reference/bin/reference',jar_sha256='aa5262a23e7f1fa50bdf9f6492e8d9f8683b2f18e2f182776966e17bdb282053',java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',source_mode='Exact stdin bytes; no trimming; -pipe -tsvg',svg_argv=['reference','--cwd','<remote>/<row.cwd>','--timezone','<row.zone>','--','-pipe','-tsvg'],probe_argv=['reference','--cwd','<remote>/<row.cwd>','--timezone','<row.zone>','--probe','<remote>/SourceProbe.java','--classpath','<remote>','--','<remote>/<row.source>','<remote>/<row.cwd>'],environment='Per-case row.env overrides are benign synthetic controls; otherwise pinned runner environment. Do not expose ambient environment.',result_requirements=['raw stdout/stderr base64 and status per SVG/probe command','actual argv/cwd/env overrides, input SHA256','whole bundle and resource checksum validation'])
(S/'remote-request.json').write_text(json.dumps(request,indent=2)+'\n')
print(len(rows), 'cases',len(resources),'resources',request['manifest_sha256'])
