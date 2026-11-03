#!/usr/bin/env python3
# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
"""Independent review fixtures and CLI/comparator parity measurements.

Run with uv run --no-project <this file> generate|generate-focused|measure OUTPUT_DIR [BIN_DIR].
BIN_DIR contains freshly built rustuml and diff_one (copies are acceptable).
Generated Java evidence is exclusive-created: existing fixtures are never replaced.
"""

import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
MECHANISM = "sequence-grouped-note-origin"
FIXTURES = ROOT / "test-diagrams/perturbations" / MECHANISM
JAVA = "/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java"
JAR = "/Users/marcelo/work/github.com/plantuml/plantuml/build/libs/plantuml-1.2026.3beta6.jar"
ENV = dict(os.environ, RUSTUML_DEBUG="date=1774210426000,tz=AEDT+1100",
           PLANTUML_STDLIB="/Users/marcelo/work/github.com/plantuml/plantuml-stdlib/stdlib",
           JAVA_HOME=str(Path(JAVA).parent.parent))
ENV["PATH"] = str(Path(JAVA).parent) + os.pathsep + ENV["PATH"]


def cases():
    result = []

    def add(name, body, axes, style="", reverse=False):
        declarations = ['participant "Iris" as I', 'participant "Wren.f" as W',
                        'participant "Kiln" as K']
        if reverse:
            declarations.reverse()
        source = "@startuml\n" + style + "\n".join(declarations) + "\n" + body + "\n@enduml\n"
        result.append(dict(name="checker_" + name, source=source, axes=axes))

    for depth in (1, 3):
        for outside in (False, True):
            for wide in (False, True):
                outer = "WMWM reservation iii" if wide else "ii.f"
                inner = "MMMM compete iii.f" if wide else "W.f"
                pre = f"note left of I : {outer}\n" if outside else ""
                body = pre + "group ledger\n"
                if not outside:
                    body += f"note left of I : {outer}\n"
                body += "loop retry\n" * (depth - 1)
                body += f"note left of I\n{inner}\nii\nend note\nI -> W : q\n"
                body += "end\n" * depth
                add(f"compete_d{depth}_{'outside' if outside else 'inside'}_{'wide' if wide else 'narrow'}",
                    body, ["competing grouped versus ungrouped note", "nesting depth", "fractional label widths"])

    for reverse in (False, True):
        for active in (False, True):
            body = "K -> I ++ : acquire\n" if active else "K -> I : acquire\n"
            body += "alt primary\nnote left of I : WWW docket f\nI -> W ++ : inner\n"
            body += "opt validation\nnote left of W\niii.f\nsecond\nend note\nreturn checked\nend\n"
            body += "else secondary\nnote left of I : i\nI -> K : closed\nend\n"
            if active:
                body += "return released\n"
            add(f"alternatives_{int(reverse)}_{int(active)}", body,
                ["participant declaration order", "nested return ownership", "alt branch note competition"],
                reverse=reverse)

    for depth in (1, 2, 3):
        body = "K -> I ++ : enter\n"
        for level in range(depth - 1):
            body += f"I -> I ++ : nested{level}\n"
        body += "critical window\nnote left of I\nFractional Wi.f envelope\nline\nend note\n"
        body += "I -> W ++ : delegate\nreturn answer\nend\n"
        for _ in range(depth):
            body += "return unwind\n"
        add(f"live_stack_{depth}", body,
            ["multiple live segment offsets", "nested self activation and return", "critical group"])

    for direction in ("[-> I", "I ->[", "K ->]", "[<- I"):
        token = {"[-> I": "incoming", "I ->[": "outgoing", "K ->]": "right", "[<- I": "reverse"}[direction]
        body = "group envelope\n" + direction + " : boundary Wi.f\n"
        body += "loop retry\nnote left of I : Wide note MMMM.f\nI -> W : transfer\nend\n"
        body += "note left of W : ii\nend\n"
        add(f"boundary_{token}", body,
            ["virtual border direction", "nested competing left notes", "fractional labels"])

    for padding in (0, 3, 7):
        for side in ("left", "right"):
            body = f"group padded\nI -> W ++ : take\nnote {side} of W\nWi.f\nwidth\nend note\n"
            body += "opt inner\nnote left of I : MMMM.f\nreturn sent\nend\nend\n"
            add(f"padding_{padding}_{side}", body,
                ["global note padding", "side-note direction", "active segment with nested note"],
                style=f"skinparam Padding {padding}\n")

    for position in ("left", "right", "over"):
        add(f"ungrouped_{position}", f"I -> W ++ : open\nnote {position} of I\nWide fractional Wi.f\nline\nend note\nreturn close",
            ["ungrouped scope control", "note position", "multiline fractional label"])
    add("teoz_control", "group teoz\nnote left of I : Wide fractional Wi.f\nloop nested\nI -> W ++ : call\nnote left of W : iii\nreturn reply\nend\nend",
        ["Teoz boundary control", "nested group", "competing active notes"], style="!pragma teoz true\n")
    return result


def focused_cases():
    result = []
    for depth in (2, 3):
        for count in (1, 2, 3, 4):
            body = "group outer\nnote left of I : MMMM" + "i" * count + "\n"
            body += "loop nested\n" * (depth - 1)
            body += "note left of I : MMMMi\nI -> W : q\n" + "end\n" * depth
            result.append(dict(name=f"checker_winner_d{depth}_i{count}",
                               source="@startuml\nparticipant Iris as I\nparticipant Wren as W\n" + body + "@enduml\n",
                               axes=["ordinary versus nested-list minimum ownership", "nearby fractional note widths", "group depth"]))
    for active in (False, True):
        body = "W -> I ++ : start\n" if active else "W -> I : start\n"
        body += "group shadow\nnote left of I : Wide shadow Wi.f\nI -> W : q\nend\n"
        if active:
            body += "return done\n"
        result.append(dict(name=f"checker_shadow_active_{int(active)}",
                           source="@startuml\nskinparam shadowing true\nparticipant Iris as I\nparticipant Wren as W\n" + body + "@enduml\n",
                           axes=["shadow component dimensions", "live versus inactive left note", "fractional label width"]))
    for font in ("Serif", "SansSerif"):
        result.append(dict(name=f"checker_font_control_{font.lower()}",
                           source=f'@startuml\nskinparam defaultFontName "{font}"\ncaption Independent font width check\nparticipant Iris\nparticipant Wren\nIris -> Wren : Fractional iii.W\n@enduml\n',
                           axes=["physical versus logical font metrics control", "fresh labels and chrome", "no grouped note control"]))
    return result


def generate(output, rows):
    FIXTURES.mkdir(parents=True, exist_ok=True)
    for row in rows:
        path = FIXTURES / (row["name"] + ".puml")
        with path.open("x") as stream:
            stream.write(row["source"])
        command = [JAVA, "-Djava.awt.headless=true", "-jar", JAR, "-tsvg", "-pipe", "-charset", "UTF-8"]
        proc = subprocess.run(command, input=row["source"], text=True, capture_output=True, env=ENV, timeout=60)
        with path.with_suffix(".svg").open("x") as stream:
            stream.write(proc.stdout)
        row["java_command"] = command
        row["java_exit"] = proc.returncode
        row["java_stderr"] = proc.stderr
        # diff_one supplies the existing error-page and strict SVG checks later.
        row["svg_sha256"] = hashlib.sha256(proc.stdout.encode()).hexdigest()
    (output / "generated.json").write_text(json.dumps(rows, indent=2) + "\n")


def measure(output, binaries):
    sources = sorted((ROOT / "test-diagrams/perturbations").rglob("*.puml"))

    def check(path):
        rel = str(path.relative_to(ROOT))
        row = dict(source=rel, cohort="added" if path.parent == FIXTURES else "preserved")
        for mode, flags in (("product", ["--no-oracle"]), ("assisted", [])):
            cmd = [str(binaries / "diff_one"), *flags, str(path)]
            p = subprocess.run(cmd, text=True, capture_output=True, env=ENV, timeout=60)
            row[mode] = dict(exit=p.returncode, stdout=p.stdout, stderr=p.stderr)
        if row["cohort"] == "added":
            command = [str(binaries / "rustuml"), "-tsvg", "-pipe"]
            cli = subprocess.run(command, input=path.read_text(), text=True, capture_output=True, env=ENV, timeout=60)
            rendered = subprocess.run([str(binaries / "diff_one"), "--no-oracle", "--print-rust", str(path)],
                                      text=True, capture_output=True, env=ENV, timeout=60)
            (output / (path.stem + ".rust.svg")).write_text(cli.stdout)
            row["cli"] = dict(command=command, exit=cli.returncode, stderr=cli.stderr,
                              product_render_exit=rendered.returncode,
                              same_svg_as_compared_product=(cli.returncode == 0 and rendered.returncode == 0
                                                            and cli.stdout.strip() == rendered.stdout.strip()),
                              sha256=hashlib.sha256(cli.stdout.encode()).hexdigest())
        return row

    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        rows = list(pool.map(check, sources))
    (output / "measurements.json").write_text(json.dumps(rows, indent=2) + "\n")
    for cohort in ("preserved", "added"):
        selected = [row for row in rows if row["cohort"] == cohort]
        for mode in ("product", "assisted"):
            counts = {code: sum(row[mode]["exit"] == code for row in selected)
                      for code in sorted({row[mode]["exit"] for row in selected})}
            print(cohort, mode, "total", len(selected), "exit_counts", counts)
        if cohort == "added":
            print("CLI equivalence", sum(row["cli"]["same_svg_as_compared_product"] and row["cli"]["exit"] == 0 for row in selected), len(selected))


if __name__ == "__main__":
    out = Path(sys.argv[2]).resolve()
    out.mkdir(parents=True, exist_ok=True)
    if sys.argv[1] == "generate":
        generate(out, cases())
    elif sys.argv[1] == "generate-focused":
        generate(out, focused_cases())
    elif sys.argv[1] == "measure":
        measure(out, Path(sys.argv[3]).resolve())
    else:
        raise SystemExit("expected generate or measure")
