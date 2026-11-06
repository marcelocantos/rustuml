# Checker round-8 (claude) relay request: java.text.Bidi differential (Java source probe)

Read-only. Runs `java.text.Bidi` (jdk.internal.icu.text.BidiBase) of the reference JVM over 242,508 inputs:
every Unicode 15.0.0 BidiCharacterTest.txt line (91,707), 150,000 seeded random strings and 801 stack/overflow
strings (`gen_inputs.py`, seed 20261106). No PlantUML classpath is needed.

Stage `BidiProbe.java` and `bidi-input.txt` in a fresh worker directory (WORKER_PATH), then:

    ssh -T -o BatchMode=yes marcelo@192.168.1.112 \
      '/Users/marcelo/rustuml-reference/bin/reference --probe WORKER_PATH/BidiProbe.java -- WORKER_PATH/bidi-input.txt WORKER_PATH/bidi-java-levels.txt' \
      > probe.stdout 2> probe.stderr; echo $? > probe.exit

Return `bidi-java-levels.txt` (first line: JVM identity), probe.stdout, probe.stderr, probe.exit, with SHA256s.

Output per input line: requiresBidi;isLeftToRight;getBaseLevel;getLevelAt(0..n-1);run levels (BidiUtils.getLevels).
The Rust prediction (`rust-tip-levels.txt`, tip c3116f0b0 java_bidi::set_para / requires_bidi /
text_line_levels, recorded before any Java run) has per line: requiresBidi;isLeftToRight;paraLevel;levels;TextLine levels or '-'.
