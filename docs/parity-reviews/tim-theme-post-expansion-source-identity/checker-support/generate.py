# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json
import os
from pathlib import Path
import subprocess

ROOT = Path('/tmp/rustuml-tim-checker')
CASES = ROOT / 'cases'
CASES.mkdir(exist_ok=True)
JAVA = '/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java'
JAR = '/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar'
env = dict(os.environ, JAVA_HOME=str(Path(JAVA).parent.parent))
env['PATH'] = str(Path(JAVA).parent) + ':' + env['PATH']
cases = {}
def add(name, body, *axes):
    cases['checker_' + name] = {'source': '@startuml\n' + body + '\n@enduml\n', 'axes': list(axes)}

add('theme_variable', '!$palette = "cerulean"\n!theme $palette\nclass MapleLedger', 'variable theme selector', 'renamed entity')
add('theme_function', '!function $palette()\n!return "minty"\n!endfunction\n!theme $palette()\nclass HazelLedger', 'function theme selector', 'theme choice')
add('theme_procedure', '!procedure $load()\n!theme cerulean\n!endprocedure\n$load()\nclass LarchLedger', 'theme in procedure', 'declaration after call')
add('theme_loop', '!foreach $n in [1,2]\n!theme cerulean\n!endfor\nclass ElmLedger', 'theme in foreach', 'repeated theme')
add('theme_while', '!$n = 0\n!while $n < 2\n!theme minty\n!$n = $n + 1\n!endwhile\nclass SpruceLedger', 'theme in while', 'mutable loop state')
add('inactive_function', '!function $tone()\n!return "#123456"\n!endfunction\n!theme cerulean\n!if 0\n!function $tone()\n!return "#654321"\n!endfunction\n!endif\nskinparam classBorderColor $tone()\nclass AcaciaLedger', 'inactive function redefinition', 'theme and custom color')
add('outline_state', '!$BGCOLOR = "#123456"\n!theme cerulean-outline\nskinparam classBackgroundColor $BGCOLOR\nclass AlderLedger', 'theme guarded variable', 'outline theme')
add('subblock_state', '!theme cerulean\n!startsub change\n!$tone = "#123456"\nskinparam classBorderColor $tone\n!endsub\nclass AspenLedger', 'user subblock execution', 'state and style')
add('sprite_identity', '!theme cerulean\nsprite $mark [2x2/16] {\nF0\n0F\n}\nclass BeechLedger', 'sprite source retained in identity', 'themed class')
add('dump_memory', '!theme cerulean\n!$tag = "maple"\n!dump_memory\nclass AshLedger', 'debug directive consumption', 'variable state')
add('inline_comment', '!theme cerulean\nclass BirchLedger \' trailing comment', 'inline comment', 'whitespace after declaration')
add('blank_lines', '\n\n!theme cerulean\n\n\nclass HawthornLedger\n\n', 'leading versus internal blank lines', 'renamed entity')
add('option_false', '!option handwritten false\nskinparam shadowing true\nclass CypressLedger', 'non-themed option', 'source-derived shadow ID')
add('option_true', '!option handwritten true\nskinparam shadowing true\nclass PoplarLedger', 'non-themed option', 'handwritten shapes and shadow')
add('option_inactive', '!theme cerulean\n!if 0\n!option handwritten true\n!endif\nclass HemlockLedger', 'inactive option', 'source-line attribution')
add('repeated_reverse', '!theme minty\n!theme cerulean\nclass RowanLedger', 'reversed theme order', 'class circled-character style')
add('inactive_theme_pair', '!theme minty\n!if 0\n!theme cerulean\n!theme united\n!endif\nclass YewLedger', 'two inactive themes', 'line attribution')
add('include_theme', '!include docs/parity-reviews/tim-theme-post-expansion-source-identity/checker-support/include-theme.iuml\nclass WillowLedger', 'theme through include', 'stateful include and declaration')
add('inactive_include', '!theme cerulean\n!if 0\n!include docs/parity-reviews/tim-theme-post-expansion-source-identity/checker-support/include-theme.iuml\n!endif\nclass OliveLedger', 'inactive include', 'theme side effects')
add('line_continuation', '!theme cerulean\nAlice -> Bob : first\\\nsecond', 'sequence line continuation', 'theme source identity')
add('theme_custom_function', '!theme cerulean\n!function $border()\n!return $PRIMARY\n!endfunction\nskinparam classBorderColor $border()\n!$PRIMARY = "#123456"\nskinparam classBackgroundColor $border()\nclass MagnoliaLedger', 'function observes theme state', 'assignment ordering')
add('clock_year', '!theme cerulean\n!$stamp = %date("yyyy")\nclass "Clock $stamp" as ClockLedger', 'environmental date evaluation', 'themed identity and label')

maker = Path('/Users/marcelo/.t3/worktrees/rustuml/t3-eac091cc/docs/parity-reviews/tim-theme-post-expansion-source-identity/maker-evidence')
for directory in sorted(maker.iterdir()):
    name = 'maker_' + directory.name
    cases[name] = {'source': (directory/'source.puml').read_text(), 'axes': ['preserved maker input', directory.name], 'maker': str(directory)}
for name, info in cases.items():
    directory = CASES / name
    directory.mkdir(exist_ok=True)
    path = directory/'source.puml'
    path.write_text(info['source'])
    if 'maker' in info:
        (directory/'preserved.svg').write_bytes((Path(info['maker'])/'source.svg').read_bytes())
    with (directory/'java.svg').open('wb') as out, (directory/'java.stderr').open('wb') as err:
        p = subprocess.run([JAVA, '-Djava.awt.headless=true', '-jar', JAR, '-pipe', '-tsvg'], input=info['source'].encode(), stdout=out, stderr=err, env=env, timeout=90)
    info['java_exit'] = p.returncode
    # Java's file -preproc produces the exact post-TIM stream, including blanks.
    p = subprocess.run([JAVA, '-Djava.awt.headless=true', '-jar', JAR, '-preproc', str(path)], capture_output=True, env=env, timeout=90)
    (directory/'preproc.stderr').write_bytes(p.stderr)
    info['preproc_exit'] = p.returncode
    expanded = path.with_suffix('.preproc')
    if expanded.exists(): expanded.rename(directory/'java.expanded')
    print(name, info['java_exit'], flush=True)
(ROOT/'cases.json').write_text(json.dumps(cases, indent=2) + '\n')
